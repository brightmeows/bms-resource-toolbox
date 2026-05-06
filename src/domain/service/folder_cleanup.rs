use std::path::Path;

use crate::domain::bms::types::MEDIA_FILE_EXTS;
use crate::domain::error::DomainError;
use crate::domain::port::{FsPort, OutputPort};

/// Service for cleaning up folders — renaming numbered directories and
/// removing zero-sized media / temp files.
pub struct FolderCleanupService {
    fs: Box<dyn FsPort>,
    output: Box<dyn OutputPort>,
}

impl FolderCleanupService {
    /// Create a new `FolderCleanupService`.
    #[must_use]
    pub fn new(fs: Box<dyn FsPort>, output: Box<dyn OutputPort>) -> Self {
        Self { fs, output }
    }

    /// Copy names from source numbered dirs to matching destination dirs.
    ///
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if filesystem operations fail.
    pub async fn copy_numbered_workdir_names(
        &self,
        root_dir_from: &Path,
        root_dir_to: &Path,
    ) -> Result<(), DomainError> {
        if !self.fs.is_dir(root_dir_from).await || !self.fs.is_dir(root_dir_to).await {
            return Ok(());
        }

        let mut src_names: Vec<(String, std::path::PathBuf)> = Vec::new();
        if let Ok(entries) = self.fs.read_dir(root_dir_from).await {
            for entry in entries {
                if !entry.is_dir {
                    continue;
                }
                src_names.push((entry.name, entry.path));
            }
        }

        let entries = self.fs.read_dir(root_dir_to).await?;
        for entry in entries {
            if !entry.is_dir {
                continue;
            }

            let dst_name = entry.name.as_str();
            let num_prefix = dst_name.split_whitespace().next().unwrap_or("");
            let numeric_part = num_prefix.split('.').next().unwrap_or("");

            if !numeric_part.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            for (src_name, _src_path) in &src_names {
                if src_name.starts_with(numeric_part) {
                    let target_path = entry.path.with_file_name(src_name);
                    if target_path != entry.path {
                        self.output.info(&format!(
                            "Renaming {:?} -> {:?}",
                            entry.path.file_name(),
                            target_path.file_name()
                        ));
                        self.fs.rename(&entry.path, &target_path).await?;
                    }
                    break;
                }
            }
        }

        Ok(())
    }

    /// Recursively remove zero-sized media files and temp files (desktop.ini,
    /// thumbs.db, `.ds_store`, `.trash-*`, `._*`).
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if filesystem operations fail.
    pub async fn remove_zero_sized_media_files(
        &self,
        start_dir: &Path,
        print_dir: bool,
    ) -> Result<(), DomainError> {
        let mut dirs_to_process = vec![start_dir.to_path_buf()];

        while let Some(current_dir) = dirs_to_process.pop() {
            if print_dir {
                self.output
                    .info(&format!("Entering dir: {}", current_dir.display()));
            }

            if !self.fs.is_dir(&current_dir).await {
                self.output.warn("Not a vaild dir! Aborting...");
                continue;
            }

            let entries = self.fs.read_dir(&current_dir).await?;

            for entry in &entries {
                let element_path = &entry.path;
                let element_name = entry.name.as_str();

                if self.fs.is_file(element_path).await {
                    let is_temp_file = element_name.to_lowercase() == "desktop.ini"
                        || element_name.to_lowercase() == "thumbs.db"
                        || element_name.to_lowercase() == ".ds_store"
                        || element_name.starts_with(".trash-")
                        || element_name.starts_with("._");

                    if is_temp_file {
                        match self.fs.remove_file(element_path).await {
                            Ok(()) => self
                                .output
                                .info(&format!(" - Remove temp file: {}", element_path.display())),
                            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                                self.output.warn(" x PermissionError!");
                            }
                            Err(_) => {}
                        }
                        continue;
                    }

                    if !MEDIA_FILE_EXTS
                        .iter()
                        .any(|ext| element_name.ends_with(ext))
                    {
                        continue;
                    }

                    match self.fs.metadata(element_path).await {
                        Ok(metadata) if metadata.len == 0 => {
                            match self.fs.remove_file(element_path).await {
                                Ok(()) => self.output.info(&format!(
                                    " - Remove empty file: {}",
                                    element_path.display()
                                )),
                                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                                    self.output.warn(" x PermissionError!");
                                }
                                Err(_) => {}
                            }
                        }
                        _ => {}
                    }
                } else if self.fs.is_dir(element_path).await {
                    dirs_to_process.push(element_path.clone());
                }
            }
        }

        Ok(())
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
    async fn test_copy_numbered_workdir_names() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        fs::create_dir_all(src.path().join("1. Song A [Artist]"))
            .await
            .unwrap();
        fs::create_dir_all(src.path().join("2. Song B [Artist]"))
            .await
            .unwrap();
        fs::create_dir_all(dst.path().join("1")).await.unwrap();
        fs::create_dir_all(dst.path().join("2")).await.unwrap();

        let service =
            FolderCleanupService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .copy_numbered_workdir_names(src.path(), dst.path())
            .await
            .unwrap();

        assert!(
            dst.path().join("1. Song A [Artist]").is_dir(),
            "should rename 1 -> 1. Song A"
        );
        assert!(
            dst.path().join("2. Song B [Artist]").is_dir(),
            "should rename 2 -> 2. Song B"
        );
        assert!(
            !dst.path().join("1").exists(),
            "original 1 should be renamed"
        );
    }

    #[tokio::test]
    async fn test_copy_numbered_workdir_skip_non_numeric_dst() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        fs::create_dir_all(src.path().join("1. Title"))
            .await
            .unwrap();
        fs::create_dir_all(dst.path().join("MySong")).await.unwrap();

        let service =
            FolderCleanupService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .copy_numbered_workdir_names(src.path(), dst.path())
            .await
            .unwrap();

        assert!(
            dst.path().join("MySong").is_dir(),
            "non-numeric dir unchanged"
        );
    }

    #[tokio::test]
    async fn test_remove_zero_sized_media() {
        let root = TempDir::new().unwrap();
        fs::write(root.path().join("empty.wav"), "").await.unwrap();
        fs::write(root.path().join("full.wav"), "data")
            .await
            .unwrap();
        fs::write(root.path().join("desktop.ini"), "")
            .await
            .unwrap();
        fs::write(root.path().join("normal.txt"), "text")
            .await
            .unwrap();

        let service =
            FolderCleanupService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .remove_zero_sized_media_files(root.path(), false)
            .await
            .unwrap();

        assert!(
            !root.path().join("empty.wav").exists(),
            "zero-sized media removed"
        );
        assert!(
            !root.path().join("desktop.ini").exists(),
            "temp file removed"
        );
        assert!(
            root.path().join("full.wav").is_file(),
            "non-zero media kept"
        );
        assert!(root.path().join("normal.txt").is_file(), "non-media kept");
    }

    #[tokio::test]
    async fn test_remove_zero_sized_recursive() {
        let root = TempDir::new().unwrap();
        let sub = root.path().join("sub");
        fs::create_dir_all(&sub).await.unwrap();
        fs::write(sub.join("empty.mp4"), "").await.unwrap();

        let service =
            FolderCleanupService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .remove_zero_sized_media_files(root.path(), false)
            .await
            .unwrap();

        assert!(!sub.join("empty.mp4").exists());
    }
}
