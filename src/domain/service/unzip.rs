use std::path::{Path, PathBuf};

use tokio::fs;

use crate::domain::bms::types::CHART_FILE_EXTS;
use crate::domain::error::DomainError;
use crate::domain::port::{FsPort, OutputPort};
use crate::infra::archive::extract::extract_archive;
use crate::infra::archive::flatten::{
    get_num_set_file_names, move_out_files_in_folder_in_cache_dir,
};
use crate::infra::fs::pack_move::is_dir_having_file;
use crate::infra::fs::utils::copy_dir_recursive;

/// Service for extracting BMS archive files (numeric and name-based modes).
pub struct UnzipService {
    fs: Box<dyn FsPort>,
    output: Box<dyn OutputPort>,
}

impl UnzipService {
    /// Create a new `UnzipService` with the given port implementations.
    #[must_use]
    pub fn new(fs: Box<dyn FsPort>, output: Box<dyn OutputPort>) -> Self {
        Self { fs, output }
    }

    /// Extract numeric-prefixed archives to BMS folder structure.
    ///
    /// Gets numbered file list, extracts to cache, finds or creates target
    /// directory with exact numeric match, moves extracted files, moves original
    /// to `BOFTTPacks/`.
    ///
    /// # Errors
    ///
    /// Returns an error if directory operations fail.
    pub async fn unzip_numeric_to_bms_folder(
        &self,
        pack: &Path,
        cache: &Path,
        root: &Path,
    ) -> Result<(), DomainError> {
        self.output.info(&format!(
            "Unzip numeric to BMS folder: {pack:?} -> {root:?}"
        ));

        if !self.fs.is_dir(cache).await {
            self.fs.create_dir_all(cache).await?;
        }
        if !self.fs.is_dir(root).await {
            self.fs.create_dir_all(root).await?;
        }

        let num_set_file_names = get_num_set_file_names(pack).await;
        self.output.info(&format!(
            "Found {} numbered pack files",
            num_set_file_names.len()
        ));

        for file_name in &num_set_file_names {
            let file_path = pack.join(file_name);
            if !self.fs.is_file(&file_path).await {
                continue;
            }

            let id_str = file_name.split_whitespace().next().unwrap_or("");
            if id_str.is_empty() {
                continue;
            }

            let cache_dir_path = cache.join(id_str);

            if self.fs.is_dir(&cache_dir_path).await && is_dir_having_file(&cache_dir_path).await {
                fs::remove_dir_all(&cache_dir_path).await?;
            }
            if !self.fs.is_dir(&cache_dir_path).await {
                self.fs.create_dir_all(&cache_dir_path).await?;
            }

            self.output
                .info(&format!("Extracting {file_path:?} to {cache_dir_path:?}"));
            extract_archive(&file_path, &cache_dir_path).await?;

            if !move_out_files_in_folder_in_cache_dir(&cache_dir_path, &CHART_FILE_EXTS).await {
                self.output
                    .info(&format!("Failed to process cache dir: {cache_dir_path:?}"));
                continue;
            }

            let mut target_dir_path: Option<PathBuf> = None;

            if let Ok(entries) = self.fs.read_dir(root).await {
                for entry in entries {
                    if !entry.is_dir {
                        continue;
                    }
                    let dir_name = entry.name.as_str();

                    if !dir_name.starts_with(id_str) {
                        continue;
                    }
                    let remaining = &dir_name[id_str.len()..];
                    if !remaining.is_empty()
                        && !remaining.starts_with('.')
                        && !remaining.starts_with(' ')
                    {
                        continue;
                    }
                    target_dir_path = Some(entry.path);
                    break;
                }
            }

            let target_dir_path = target_dir_path.unwrap_or_else(|| root.join(id_str));

            self.move_cache_to_bms_dir(&cache_dir_path, &target_dir_path)
                .await?;
            let _ = self.fs.remove_dir(&cache_dir_path).await;

            self.output
                .info(&format!("Finished processing: {file_name}"));
            let used_pack_dir = pack.join("BOFTTPacks");
            if !self.fs.is_dir(&used_pack_dir).await {
                self.fs.create_dir_all(&used_pack_dir).await?;
            }
            let target_file_path = used_pack_dir.join(file_name);
            let _ = self.fs.rename(&file_path, &target_file_path).await;
        }

        Ok(())
    }

    /// Extract archives by original filename to BMS folder structure.
    ///
    /// # Errors
    ///
    /// Returns an error if directory operations fail.
    pub async fn unzip_with_name_to_bms_folder(
        &self,
        pack: &Path,
        cache: &Path,
        root: &Path,
    ) -> Result<(), DomainError> {
        self.output.info(&format!(
            "Unzip with name to BMS folder: {pack:?} -> {root:?}"
        ));

        self.create_dirs(cache, root).await?;
        let archive_names = self.get_archive_files(pack).await;

        if archive_names.is_empty() {
            self.output
                .info(&format!("No archive files found in {pack:?}"));
            return Ok(());
        }

        for file_name in &archive_names {
            self.process_single_archive(pack, cache, root, file_name)
                .await?;
        }

        Ok(())
    }

    /// Set a file number for a single file in a directory.
    ///
    /// Renames `file_idx`-th numbered file to have prefix `num`.
    ///
    /// # Errors
    ///
    /// Returns an error if directory operations fail.
    pub async fn set_file_num(
        &self,
        path: &Path,
        file_idx: usize,
        num: i32,
    ) -> Result<(), DomainError> {
        const ALLOWED_EXTS: &[&str] = &["zip", "7z", "rar", "mp4", "bms", "bme", "bml", "pms"];

        self.output
            .info(&format!("Setting file numbers in: {path:?}"));

        let Ok(entries) = self.fs.read_dir(path).await else {
            return Ok(());
        };

        let mut file_names: Vec<String> = Vec::new();

        for entry in entries {
            if !self.fs.is_file(&entry.path).await {
                continue;
            }

            let name = entry.name.as_str();

            if name
                .split_whitespace()
                .next()
                .is_some_and(|s| s.chars().all(|c| c.is_ascii_digit()))
            {
                continue;
            }

            let part_file_path = entry.path.with_file_name(format!("{name}.part"));
            if self.fs.is_file(&part_file_path).await {
                continue;
            }

            if self
                .fs
                .metadata(&entry.path)
                .await
                .map_or(true, |m| m.len == 0)
            {
                continue;
            }

            let ext = name.rsplit('.').next().unwrap_or("");
            if !ALLOWED_EXTS.contains(&ext) {
                continue;
            }

            file_names.push(name.to_string());
        }

        if file_names.is_empty() {
            self.output.info("No files to number");
            return Ok(());
        }

        self.output
            .info(&format!("Here are files in {}:", path.display()));
        for (i, name) in file_names.iter().enumerate() {
            self.output.info(&format!(" - {i}: {name}"));
        }

        let Some(file_name) = file_names.get(file_idx) else {
            self.output.info(&format!(
                "Invalid file index {file_idx}, max is {}",
                file_names.len() - 1
            ));
            return Ok(());
        };

        let file_path = path.join(file_name);
        let new_file_name = format!("{num} {file_name}");
        let new_file_path = path.join(&new_file_name);

        self.output
            .info(&format!("Rename {file_name} to {new_file_name}"));
        self.fs.rename(&file_path, &new_file_path).await?;

        Ok(())
    }

    /// Move extracted cache files to the target BMS directory.
    /// Falls back to copy+delete when cross-device rename fails.
    async fn move_cache_to_bms_dir(
        &self,
        cache_dir_path: &Path,
        target_dir_path: &Path,
    ) -> Result<(), DomainError> {
        self.output.info(&format!(
            "Moving files from {cache_dir_path:?} to {target_dir_path:?}"
        ));

        let entries = self.fs.read_dir(cache_dir_path).await?;

        self.fs.create_dir_all(target_dir_path).await?;

        for entry in entries {
            let src_path = entry.path;
            let dst_path = target_dir_path.join(&entry.name);
            if self.fs.rename(&src_path, &dst_path).await.is_err() {
                if entry.is_dir {
                    copy_dir_recursive(&src_path, &dst_path).await?;
                    fs::remove_dir_all(&src_path).await?;
                } else {
                    fs::copy(&src_path, &dst_path).await?;
                    self.fs.remove_file(&src_path).await?;
                }
            }
        }

        Ok(())
    }

    async fn create_dirs(&self, cache: &Path, root: &Path) -> Result<(), DomainError> {
        if !self.fs.is_dir(cache).await {
            self.fs.create_dir_all(cache).await?;
        }
        if !self.fs.is_dir(root).await {
            self.fs.create_dir_all(root).await?;
        }
        Ok(())
    }

    async fn get_archive_files(&self, pack_dir: &Path) -> Vec<String> {
        #[expect(clippy::case_sensitive_file_extension_comparisons)]
        let check_ext = |n: &str| n.ends_with(".zip") || n.ends_with(".7z") || n.ends_with(".rar");

        let Ok(entries) = self.fs.read_dir(pack_dir).await else {
            return Vec::new();
        };

        entries
            .into_iter()
            .filter(|e| e.is_file)
            .map(|e| e.name)
            .filter(|n| check_ext(n))
            .collect()
    }

    async fn process_single_archive(
        &self,
        pack_dir: &Path,
        cache_dir: &Path,
        root_dir: &Path,
        file_name: &str,
    ) -> Result<(), DomainError> {
        let file_path = pack_dir.join(file_name);

        let file_stem = file_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .trim_end_matches('.')
            .to_string();

        let cache_dir_path = cache_dir.join(&file_stem);
        self.prepare_cache_directory(&cache_dir_path).await?;

        extract_archive(&file_path, &cache_dir_path).await?;

        if !move_out_files_in_folder_in_cache_dir(&cache_dir_path, &CHART_FILE_EXTS).await {
            self.output
                .info(&format!("Failed to process cache dir: {cache_dir_path:?}"));
            return Ok(());
        }

        let target_dir_path = root_dir.join(&file_stem);
        self.move_cache_to_bms_dir(&cache_dir_path, &target_dir_path)
            .await?;
        let _ = self.fs.remove_dir(&cache_dir_path).await;
        self.move_original_to_bofttpacks(&file_path, pack_dir, file_name)
            .await;

        self.output
            .info(&format!("Finished processing: {file_name}"));
        Ok(())
    }

    async fn prepare_cache_directory(&self, cache_dir_path: &Path) -> Result<(), DomainError> {
        if self.fs.is_dir(cache_dir_path).await {
            let has_files = {
                let Ok(entries) = self.fs.read_dir(cache_dir_path).await else {
                    return Ok(());
                };
                entries.iter().any(|e| e.is_file)
            };
            if has_files {
                self.output
                    .info(&format!("Removing existing cache dir: {cache_dir_path:?}"));
                fs::remove_dir_all(cache_dir_path).await?;
            }
        }
        self.fs.create_dir_all(cache_dir_path).await?;
        Ok(())
    }

    async fn move_original_to_bofttpacks(
        &self,
        file_path: &Path,
        pack_dir: &Path,
        file_name: &str,
    ) {
        let used_pack_dir = pack_dir.join("BOFTTPacks");
        if !self.fs.is_dir(&used_pack_dir).await {
            let _ = self.fs.create_dir_all(&used_pack_dir).await;
        }
        let target_file_path = used_pack_dir.join(file_name);
        let _ = self.fs.rename(file_path, &target_file_path).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::service::test_util::MockOutput;
    use crate::infra::adapters::fs::TokioFsAdapter;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_set_file_num_basic() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("song.bms"), "#TITLE Song\n")
            .await
            .unwrap();
        let service = UnzipService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.set_file_num(dir.path(), 0, 42).await.unwrap();
        assert!(
            dir.path().join("42 song.bms").is_file(),
            "should prefix with num"
        );
        assert!(
            !dir.path().join("song.bms").exists(),
            "original should be renamed"
        );
    }

    #[tokio::test]
    async fn test_set_file_num_skips_already_numbered() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("10 song.bms"), "#TITLE Song\n")
            .await
            .unwrap();
        let service = UnzipService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.set_file_num(dir.path(), 0, 99).await.unwrap();
        assert!(
            dir.path().join("10 song.bms").is_file(),
            "already numbered should be skipped"
        );
    }

    #[tokio::test]
    async fn test_set_file_num_skips_empty_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("song.bms"), "").await.unwrap();
        let service = UnzipService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.set_file_num(dir.path(), 0, 1).await.unwrap();
        assert!(dir.path().join("song.bms").is_file());
    }

    #[tokio::test]
    async fn test_unzip_with_name_creates_dirs() {
        let pack = TempDir::new().unwrap();
        let cache = TempDir::new().unwrap();
        let root = TempDir::new().unwrap();
        let zip_path = pack.path().join("TestPack.zip");
        let file = fs::File::create(&zip_path).await.unwrap().into_std().await;
        let mut zip = zip::ZipWriter::new(file);
        zip.add_directory::<_, ()>("song/", zip::write::FileOptions::default())
            .unwrap();
        zip.finish().unwrap();
        let service = UnzipService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .unzip_with_name_to_bms_folder(pack.path(), cache.path(), root.path())
            .await
            .unwrap();
    }
}
