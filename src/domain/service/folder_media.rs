use std::collections::HashMap;
use std::path::Path;

use crate::domain::bms::types::MEDIA_FILE_EXTS;
use crate::domain::error::DomainError;
use crate::domain::port::{FsPort, OutputPort};

/// Media file removal rule
pub type RemoveMediaRule = Vec<(Vec<&'static str>, Vec<&'static str>)>;

/// ORAJA removal rule
#[must_use]
pub fn get_remove_media_rule_oraja() -> RemoveMediaRule {
    vec![
        (vec!["mp4"], vec!["avi", "wmv", "mpg", "mpeg"]),
        (vec!["avi"], vec!["wmv", "mpg", "mpeg"]),
        (vec!["flac", "wav"], vec!["ogg"]),
        (vec!["flac"], vec!["wav"]),
        (vec!["mpg"], vec!["wmv"]),
    ]
}

/// Service for removing redundant media files from folders based on priority
/// rules (e.g. keep mp4, remove avi/wmv).
pub struct FolderMediaService {
    fs: Box<dyn FsPort>,
    output: Box<dyn OutputPort>,
}

impl FolderMediaService {
    /// Create a new `FolderMediaService`.
    #[must_use]
    pub fn new(fs: Box<dyn FsPort>, output: Box<dyn OutputPort>) -> Self {
        Self { fs, output }
    }

    /// Remove redundant media files in each workdir under `root_dir` according
    /// to the given rule, then clean up zero-sized / temp files.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if filesystem operations fail.
    pub async fn remove_unneed_media_files(
        &self,
        root_dir: &Path,
        rule: RemoveMediaRule,
    ) -> Result<(), DomainError> {
        self.output.info(&format!("Selected: {rule:?}"));

        let entries = self.fs.read_dir(root_dir).await?;
        for entry in entries {
            if !entry.is_dir {
                continue;
            }
            self.workdir_remove_unneed_media_files(&entry.path, &rule)
                .await?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    async fn workdir_remove_unneed_media_files(
        &self,
        work_dir: &Path,
        rule: &RemoveMediaRule,
    ) -> Result<(), DomainError> {
        let mut remove_pairs: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();
        let mut removed_files: std::collections::HashSet<std::path::PathBuf> =
            std::collections::HashSet::new();

        let entries = self.fs.read_dir(work_dir).await?;

        for entry in &entries {
            let check_file_path = &entry.path;
            if !self.fs.is_file(check_file_path).await {
                continue;
            }

            let file_ext = check_file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            for (upper_exts, lower_exts) in rule {
                if !upper_exts.contains(&file_ext) {
                    continue;
                }

                if self
                    .fs
                    .metadata(check_file_path)
                    .await
                    .is_ok_and(|m| m.len == 0)
                {
                    self.output.info(&format!(
                        " - !x!: File {check_file_path:?} is Empty! Skipping..."
                    ));
                    continue;
                }

                for lower_ext in lower_exts {
                    let replacing_file_path = check_file_path.with_extension(lower_ext);
                    if !self.fs.is_file(&replacing_file_path).await {
                        continue;
                    }
                    if removed_files.contains(&replacing_file_path) {
                        continue;
                    }
                    remove_pairs.push((check_file_path.clone(), replacing_file_path.clone()));
                    removed_files.insert(replacing_file_path);
                }
            }
        }

        for (check_file_path, replacing_file_path) in &remove_pairs {
            self.output.info(&format!(
                "- Remove file {:?}, because {:?} exists.",
                replacing_file_path.file_name(),
                check_file_path.file_name()
            ));
            let _ = self.fs.remove_file(replacing_file_path).await;
        }

        // Remove zero-sized media and temp files recursively
        let mut cleanup_dirs = vec![work_dir.to_path_buf()];
        while let Some(dir) = cleanup_dirs.pop() {
            if !self.fs.is_dir(&dir).await {
                continue;
            }
            let de = self.fs.read_dir(&dir).await?;
            for e in de {
                let p = &e.path;
                let n = e.name.as_str();
                if self.fs.is_file(p).await {
                    let is_temp = matches!(
                        n.to_lowercase().as_str(),
                        "desktop.ini" | "thumbs.db" | ".ds_store"
                    ) || n.starts_with(".trash-")
                        || n.starts_with("._");
                    if is_temp {
                        match self.fs.remove_file(p).await {
                            Ok(()) => self
                                .output
                                .info(&format!(" - Remove temp file: {}", p.display())),
                            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                                self.output.warn(" x PermissionError!");
                            }
                            Err(_) => {}
                        }
                        continue;
                    }
                    if MEDIA_FILE_EXTS.iter().any(|ext| n.ends_with(ext))
                        && let Ok(meta) = self.fs.metadata(p).await
                        && meta.len == 0
                    {
                        match self.fs.remove_file(p).await {
                            Ok(()) => self
                                .output
                                .info(&format!(" - Remove empty file: {}", p.display())),
                            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                                self.output.warn(" x PermissionError!");
                            }
                            Err(_) => {}
                        }
                    }
                } else if self.fs.is_dir(p).await {
                    cleanup_dirs.push(p.clone());
                }
            }
        }

        let mut ext_count: HashMap<String, Vec<String>> = HashMap::new();
        let count_entries = self.fs.read_dir(work_dir).await?;
        for entry in count_entries {
            if !self.fs.is_file(&entry.path).await {
                continue;
            }
            let file_ext = entry
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            ext_count
                .entry(file_ext.to_string())
                .or_default()
                .push(entry.name);
        }

        if let Some(mp4_files) = ext_count.get("mp4")
            && mp4_files.len() > 1
        {
            self.output
                .info(&format!(" - Tips: {work_dir:?} has more than 1 mp4 files!"));
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

    async fn workdir_path(root: &Path) -> std::path::PathBuf {
        let wd = root.join("work");
        fs::create_dir_all(&wd).await.unwrap();
        wd
    }

    #[tokio::test]
    async fn test_remove_media_mp4_removes_avi() {
        let dir = TempDir::new().unwrap();
        let wd = workdir_path(dir.path()).await;
        fs::write(wd.join("v.mp4"), "mp4").await.unwrap();
        fs::write(wd.join("v.avi"), "avi").await.unwrap();
        let service =
            FolderMediaService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .remove_unneed_media_files(dir.path(), get_remove_media_rule_oraja())
            .await
            .unwrap();
        assert!(!wd.join("v.avi").exists());
    }

    #[tokio::test]
    async fn test_remove_media_flac_removes_wav() {
        let dir = TempDir::new().unwrap();
        let wd = workdir_path(dir.path()).await;
        fs::write(wd.join("a.flac"), "flac").await.unwrap();
        fs::write(wd.join("a.wav"), "wav").await.unwrap();
        let service =
            FolderMediaService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .remove_unneed_media_files(dir.path(), get_remove_media_rule_oraja())
            .await
            .unwrap();
        assert!(!wd.join("a.wav").exists());
    }

    #[tokio::test]
    async fn test_remove_media_flac_wav_removes_ogg() {
        let dir = TempDir::new().unwrap();
        let wd = workdir_path(dir.path()).await;
        fs::write(wd.join("a.flac"), "flac").await.unwrap();
        fs::write(wd.join("a.ogg"), "ogg").await.unwrap();
        let service =
            FolderMediaService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .remove_unneed_media_files(dir.path(), get_remove_media_rule_oraja())
            .await
            .unwrap();
        assert!(!wd.join("a.ogg").exists());
    }

    #[tokio::test]
    async fn test_remove_media_mp4_removes_wmv() {
        let dir = TempDir::new().unwrap();
        let wd = workdir_path(dir.path()).await;
        fs::write(wd.join("v.mp4"), "mp4").await.unwrap();
        fs::write(wd.join("v.wmv"), "wmv").await.unwrap();
        let service =
            FolderMediaService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .remove_unneed_media_files(dir.path(), get_remove_media_rule_oraja())
            .await
            .unwrap();
        assert!(!wd.join("v.wmv").exists());
    }

    #[tokio::test]
    async fn test_remove_media_no_duplicate_no_removal() {
        let dir = TempDir::new().unwrap();
        let wd = workdir_path(dir.path()).await;
        fs::write(wd.join("a.flac"), "flac").await.unwrap();
        let service =
            FolderMediaService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .remove_unneed_media_files(dir.path(), get_remove_media_rule_oraja())
            .await
            .unwrap();
        assert!(wd.join("a.flac").is_file());
    }
}
