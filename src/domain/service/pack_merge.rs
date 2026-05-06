use std::path::Path;

use crate::domain::error::DomainError;
use crate::domain::port::{FsPort, OutputPort};
use crate::infra::fs::pack_move::{MoveOptions, ReplaceOptions, move_elements_across_dir};

/// Service for merging BMS packs.
pub struct PackMergeService {
    fs: Box<dyn FsPort>,
    output: Box<dyn OutputPort>,
}

impl PackMergeService {
    /// Create a new `PackMergeService`.
    #[must_use]
    pub fn new(fs: Box<dyn FsPort>, output: Box<dyn OutputPort>) -> Self {
        Self { fs, output }
    }

    /// Merge split folders back together.
    ///
    /// This reverses the `split_folders_with_first_char` operation by moving
    /// contents from single-character-named subdirectories back to the parent.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if directory operations fail.
    pub async fn merge_split_folders(&self, root_dir: &Path) -> Result<(), DomainError> {
        let entries = self.fs.read_dir(root_dir).await?;

        let dirs: Vec<_> = entries.iter().filter(|e| e.is_dir).collect();
        let dir_names: Vec<String> = dirs.iter().map(|e| e.name.clone()).collect();

        let mut pairs: Vec<(String, String)> = Vec::new();

        for dir_name in &dir_names {
            let dir_path = root_dir.join(dir_name);
            if !self.fs.is_dir(&dir_path).await {
                continue;
            }

            if dir_name.ends_with(']') {
                let Some(dir_name_mps_i) = dir_name.rfind('[') else {
                    continue;
                };
                let dir_name_without_artist = &dir_name[..dir_name_mps_i - 1];
                if dir_name_without_artist.is_empty() {
                    continue;
                }

                let dir_path_without_artist = root_dir.join(dir_name_without_artist);
                if !self.fs.is_dir(&dir_path_without_artist).await {
                    continue;
                }

                let dir_names_with_starter: Vec<&String> = dir_names
                    .iter()
                    .filter(|d| d.starts_with(&format!("{dir_name_without_artist} [")))
                    .collect();

                if dir_names_with_starter.len() > 2 {
                    self.output.info(&format!(
                        " !_! {dir_name_without_artist} have more then 2 folders! {dir_names_with_starter:?}"
                    ));
                    continue;
                }

                pairs.push((dir_name_without_artist.to_string(), dir_name.clone()));
            }
        }

        let mut last_from_dir_name = String::new();
        let mut duplicate_list: Vec<String> = Vec::new();
        for (_target_dir_name, from_dir_name) in &pairs {
            if last_from_dir_name == *from_dir_name {
                duplicate_list.push(from_dir_name.clone());
            }
            last_from_dir_name.clone_from(from_dir_name);
        }

        if !duplicate_list.is_empty() {
            for name in &duplicate_list {
                self.output.info(&format!("Duplicate! -> {name}"));
            }
            return Err(DomainError::Archive(anyhow::anyhow!(
                "Found duplicate target directories: {duplicate_list:?}"
            )));
        }

        for (target_dir_name, from_dir_name) in &pairs {
            self.output.info(&format!(
                "- Find Dir pair: {target_dir_name} <- {from_dir_name}"
            ));
        }

        for (target_dir_name, from_dir_name) in &pairs {
            let from_dir_path = root_dir.join(from_dir_name);
            let target_dir_path = root_dir.join(target_dir_name);
            self.output
                .info(&format!(" - Moving: {target_dir_name} <- {from_dir_name}"));
            move_elements_across_dir(
                &from_dir_path,
                &target_dir_path,
                MoveOptions::default(),
                &ReplaceOptions::default(),
            )
            .await?;
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
    async fn test_merge_split_folders() {
        let root = TempDir::new().unwrap();
        let cat_dir = root.path().join("Song [Artist]");
        let base_dir = root.path().join("Song");
        fs::create_dir_all(&cat_dir).await.unwrap();
        fs::write(cat_dir.join("song.bms"), "#TITLE Song\n")
            .await
            .unwrap();
        fs::create_dir_all(&base_dir).await.unwrap();
        fs::write(base_dir.join("readme.txt"), "info")
            .await
            .unwrap();

        let service = PackMergeService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.merge_split_folders(root.path()).await.unwrap();

        assert!(base_dir.join("song.bms").is_file());
    }
}
