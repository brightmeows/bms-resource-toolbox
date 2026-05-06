use std::path::Path;

use crate::domain::error::DomainError;
use crate::domain::port::{FsPort, OutputPort};
use crate::infra::fs::pack_move::{
    MoveOptions, REPLACE_OPTION_UPDATE_PACK, is_dir_having_file, move_elements_across_dir,
};

/// Service for moving BMS packs.
pub struct PackMoveService {
    fs: Box<dyn FsPort>,
    output: Box<dyn OutputPort>,
}

impl PackMoveService {
    /// Create a new `PackMoveService`.
    #[must_use]
    pub fn new(fs: Box<dyn FsPort>, output: Box<dyn OutputPort>) -> Self {
        Self { fs, output }
    }

    /// Move works from one pack directory to another.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if directory operations fail.
    pub async fn move_works_in_pack(
        &self,
        root_dir_from: &Path,
        root_dir_to: &Path,
    ) -> Result<(), DomainError> {
        if root_dir_from == root_dir_to {
            return Ok(());
        }

        let mut move_count = 0;

        if let Ok(entries) = self.fs.read_dir(root_dir_from).await {
            for entry in &entries {
                if !entry.is_dir {
                    continue;
                }

                self.output.info(&format!("Moving: {}", entry.name));

                let dst_bms_dir = root_dir_to.join(&entry.name);
                move_elements_across_dir(
                    &entry.path,
                    &dst_bms_dir,
                    MoveOptions::default(),
                    &REPLACE_OPTION_UPDATE_PACK,
                )
                .await?;
                move_count += 1;
            }
        }

        if move_count > 0 {
            self.output.info(&format!("Move {move_count} songs."));
            return Ok(());
        }

        move_elements_across_dir(
            root_dir_from,
            root_dir_to,
            MoveOptions::default(),
            &REPLACE_OPTION_UPDATE_PACK,
        )
        .await?;

        Ok(())
    }

    /// Move works out one level (un-nest subdirectories).
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if directory operations fail.
    pub async fn move_out_works(&self, target_root_dir: &Path) -> Result<(), DomainError> {
        let entries = self.fs.read_dir(target_root_dir).await?;

        for entry in &entries {
            if !entry.is_dir {
                continue;
            }

            if let Ok(work_entries) = self.fs.read_dir(&entry.path).await {
                for work_entry in &work_entries {
                    let target_work_dir_path = target_root_dir.join(&work_entry.name);

                    move_elements_across_dir(
                        &work_entry.path,
                        &target_work_dir_path,
                        MoveOptions::default(),
                        &REPLACE_OPTION_UPDATE_PACK,
                    )
                    .await?;
                }
            }

            if !is_dir_having_file(&entry.path).await {
                let _ = self.fs.remove_dir(&entry.path).await;
            }
        }

        Ok(())
    }

    /// Move works with same name from one dir to another.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if directory operations fail.
    pub async fn move_works_with_same_name(
        &self,
        root_dir_from: &Path,
        root_dir_to: &Path,
    ) -> Result<(), DomainError> {
        if !self.fs.is_dir(root_dir_from).await {
            return Err(DomainError::Archive(anyhow::anyhow!(
                "源路径不存在或不是目录: {}",
                root_dir_from.display()
            )));
        }
        if !self.fs.is_dir(root_dir_to).await {
            return Err(DomainError::Archive(anyhow::anyhow!(
                "目标路径不存在或不是目录: {}",
                root_dir_to.display()
            )));
        }

        let mut from_subdirs: Vec<(String, std::path::PathBuf)> = Vec::new();
        if let Ok(entries) = self.fs.read_dir(root_dir_from).await {
            for entry in &entries {
                if !entry.is_dir {
                    continue;
                }
                from_subdirs.push((entry.name.clone(), entry.path.clone()));
            }
        }

        let mut to_subdirs: Vec<String> = Vec::new();
        if let Ok(entries) = self.fs.read_dir(root_dir_to).await {
            for entry in &entries {
                if !entry.is_dir {
                    continue;
                }
                to_subdirs.push(entry.name.clone());
            }
        }

        let mut pairs: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();

        for (from_name, from_path) in &from_subdirs {
            for to_name in &to_subdirs {
                if to_name.starts_with(from_name) {
                    let to_path = root_dir_to.join(to_name);
                    self.output.info(&format!(" -> {from_name} => {to_name}"));
                    pairs.push((from_path.clone(), to_path));
                    break;
                }
            }
        }

        if pairs.is_empty() {
            return Ok(());
        }

        for (from_path, to_path) in &pairs {
            self.output.info(&format!(
                "合并: {} -> {}",
                from_path.display(),
                to_path.display()
            ));
            move_elements_across_dir(
                from_path,
                to_path,
                MoveOptions::default(),
                &REPLACE_OPTION_UPDATE_PACK,
            )
            .await?;
        }

        Ok(())
    }

    /// Move works with same name to sibling directories.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if directory operations fail.
    pub async fn move_works_with_same_name_to_siblings(
        &self,
        root_dir_from: &Path,
    ) -> Result<(), DomainError> {
        if !self.fs.is_dir(root_dir_from).await {
            return Err(DomainError::Archive(anyhow::anyhow!(
                "源路径不存在或不是目录: {}",
                root_dir_from.display()
            )));
        }

        let Some(parent_dir) = root_dir_from.parent() else {
            return Ok(());
        };

        let root_base_name = root_dir_from
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let mut from_subdirs: Vec<(String, std::path::PathBuf)> = Vec::new();
        if let Ok(entries) = self.fs.read_dir(root_dir_from).await {
            for entry in &entries {
                if !entry.is_dir {
                    continue;
                }
                from_subdirs.push((entry.name.clone(), entry.path.clone()));
            }
        }

        let mut pairs: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();

        if let Ok(sibling_entries) = self.fs.read_dir(parent_dir).await {
            for sibling in &sibling_entries {
                if !sibling.is_dir {
                    continue;
                }

                if sibling.name == root_base_name {
                    continue;
                }

                let mut to_subdirs: Vec<String> = Vec::new();
                if let Ok(sub_entries) = self.fs.read_dir(&sibling.path).await {
                    for sub in &sub_entries {
                        if !sub.is_dir {
                            continue;
                        }
                        to_subdirs.push(sub.name.clone());
                    }
                }

                for (from_name, from_path) in &from_subdirs {
                    for to_name in &to_subdirs {
                        if to_name.starts_with(from_name) {
                            let target_path = sibling.path.join(to_name);
                            self.output
                                .info(&format!(" -> {from_name} => {}", target_path.display()));
                            pairs.push((from_path.clone(), target_path));
                            break;
                        }
                    }
                }
            }
        }

        if pairs.is_empty() {
            return Ok(());
        }

        for (from_path, target_path) in &pairs {
            self.output.info(&format!(
                "合并: {} -> {}",
                from_path.display(),
                target_path.display()
            ));
            move_elements_across_dir(
                from_path,
                target_path,
                MoveOptions::default(),
                &REPLACE_OPTION_UPDATE_PACK,
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
    async fn test_move_works_in_pack() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        fs::create_dir_all(src.path().join("Song1")).await.unwrap();
        fs::write(src.path().join("Song1/test.bms"), "#TITLE Song1\n")
            .await
            .unwrap();

        let service = PackMoveService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .move_works_in_pack(src.path(), dst.path())
            .await
            .unwrap();

        assert!(dst.path().join("Song1/test.bms").is_file());
    }

    #[tokio::test]
    async fn test_move_out_works() {
        let root = TempDir::new().unwrap();
        let sub = root.path().join("SubPack");
        let work = sub.join("Work1");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(work.join("test.bms"), "#TITLE Work1\n")
            .await
            .unwrap();

        let service = PackMoveService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.move_out_works(root.path()).await.unwrap();

        assert!(
            root.path().join("Work1/test.bms").is_file(),
            "work should be one level up"
        );
    }

    #[tokio::test]
    async fn test_move_works_with_same_name() {
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();
        fs::create_dir_all(src.path().join("Song1")).await.unwrap();
        fs::write(src.path().join("Song1/test.bms"), "#TITLE Song1\n")
            .await
            .unwrap();
        fs::create_dir_all(dst.path().join("Song1 [Artist]"))
            .await
            .unwrap();
        fs::write(dst.path().join("Song1 [Artist]/readme.txt"), "info")
            .await
            .unwrap();

        let service = PackMoveService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .move_works_with_same_name(src.path(), dst.path())
            .await
            .unwrap();

        assert!(
            dst.path().join("Song1 [Artist]/test.bms").is_file(),
            "bms should be merged"
        );
    }

    #[tokio::test]
    async fn test_move_works_with_same_name_to_siblings() {
        let parent = TempDir::new().unwrap();
        let src = parent.path().join("SourcePack");
        fs::create_dir_all(&src).await.unwrap();
        let sibling = parent.path().join("SiblingPack");
        fs::create_dir_all(&sibling).await.unwrap();
        fs::create_dir_all(src.join("Song1")).await.unwrap();
        fs::write(src.join("Song1/test.bms"), "#TITLE Song1\n")
            .await
            .unwrap();
        fs::create_dir_all(sibling.join("Song1 [Artist]"))
            .await
            .unwrap();

        let service = PackMoveService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .move_works_with_same_name_to_siblings(&src)
            .await
            .unwrap();

        assert!(
            sibling.join("Song1 [Artist]/test.bms").is_file(),
            "song should move to sibling"
        );
    }
}
