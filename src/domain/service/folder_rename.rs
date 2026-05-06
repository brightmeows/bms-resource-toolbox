use std::path::{Path, PathBuf};

use crate::domain::bms::info::get_dir_bms_info;
use crate::domain::error::DomainError;
use crate::domain::port::{FsPort, OutputPort};
use crate::infra::fs::name::{bms_dir_similarity, get_valid_fs_name};
use crate::infra::fs::pack_move::{
    MoveOptions, REPLACE_OPTION_UPDATE_PACK, ReplaceOptions, move_elements_across_dir,
};

/// Service for renaming folders based on BMS metadata (title, artist).
pub struct FolderRenameService {
    fs: Box<dyn FsPort>,
    output: Box<dyn OutputPort>,
}

impl FolderRenameService {
    /// Create a new `FolderRenameService`.
    #[must_use]
    pub fn new(fs: Box<dyn FsPort>, output: Box<dyn OutputPort>) -> Self {
        Self { fs, output }
    }

    /// Append `{title} [{artist}]` to directories whose name is a bare number.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if filesystem operations fail.
    pub async fn append_name_by_bms(&self, root_dir: &Path) -> Result<(), DomainError> {
        if !self.fs.is_dir(root_dir).await {
            return Ok(());
        }

        let mut to_rename: Vec<(PathBuf, PathBuf)> = Vec::new();

        let entries = self.fs.read_dir(root_dir).await?;
        for entry in entries {
            if !entry.is_dir {
                continue;
            }

            let dir_name = entry.name.as_str();

            if !dir_name.trim().is_empty()
                && dir_name
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('\u{FF10}'..='\u{FF19}').contains(&c))
                && let Some(new_name) = self.rename_folder_by_bms(&entry.path).await
            {
                let new_path = entry.path.with_file_name(&new_name);
                to_rename.push((entry.path, new_path));
            }
        }

        for (from, to) in to_rename {
            self.output.info(&format!(
                "Renaming {:?} -> {:?}",
                from.file_name(),
                to.file_name()
            ));
            self.fs.rename(&from, &to).await?;
        }

        Ok(())
    }

    #[must_use]
    async fn rename_folder_by_bms(&self, work_dir: &Path) -> Option<String> {
        let dir_name = work_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if !dir_name.trim().is_empty()
            && !dir_name
                .chars()
                .all(|c| c.is_ascii_digit() || ('\u{FF10}'..='\u{FF19}').contains(&c))
        {
            return None;
        }

        let info = get_dir_bms_info(&*self.fs, work_dir).await?;
        if info.title.is_empty() && info.artist.is_empty() {
            return None;
        }

        let new_dir_name = format!(
            "{}. {} [{}]",
            dir_name.trim(),
            get_valid_fs_name(&info.title),
            get_valid_fs_name(&info.artist)
        );

        Some(new_dir_name)
    }

    /// Append `[{artist}]` to directories that don't already end with `]`.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if filesystem operations fail.
    pub async fn append_artist_name_by_bms(&self, root_dir: &Path) -> Result<(), DomainError> {
        if !self.fs.is_dir(root_dir).await {
            return Ok(());
        }

        let mut pairs: Vec<(PathBuf, PathBuf)> = Vec::new();
        let entries = self.fs.read_dir(root_dir).await?;
        for entry in entries {
            if !entry.is_dir {
                continue;
            }

            let dir_name = entry.name.as_str();

            if dir_name.ends_with(']') {
                continue;
            }

            let info = get_dir_bms_info(&*self.fs, &entry.path).await;
            let Some(info) = info else {
                self.output
                    .info(&format!("Dir {} has no bms files!", entry.path.display()));
                continue;
            };

            let new_dir_name = format!("{dir_name} [{}]", get_valid_fs_name(&info.artist));
            self.output
                .info(&format!("- Ready to rename: {dir_name} -> {new_dir_name}"));
            pairs.push((entry.path, root_dir.join(&new_dir_name)));
        }

        if pairs.is_empty() {
            self.output.info("No folders to rename");
            return Ok(());
        }

        for (from, to) in pairs {
            self.fs.rename(&from, &to).await?;
        }

        Ok(())
    }

    /// Rename all subdirectories to `{title} [{artist}]` based on BMS metadata.
    /// Merges with existing target dirs if similarity is high.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if filesystem operations fail.
    pub async fn set_name_by_bms(&self, root_dir: &Path) -> Result<(), DomainError> {
        if !self.fs.is_dir(root_dir).await {
            return Ok(());
        }

        let mut fail_list: Vec<String> = Vec::new();

        let entries = self.fs.read_dir(root_dir).await?;
        for entry in entries {
            if !entry.is_dir {
                continue;
            }

            let dir_name = entry.name.clone();

            if !self.set_single_folder_name_by_bms(&entry.path).await? {
                fail_list.push(dir_name);
            }
        }

        if !fail_list.is_empty() {
            self.output
                .info(&format!("Fail Count: {}", fail_list.len()));
            for name in &fail_list {
                self.output.info(&format!("  {name}"));
            }
        }

        Ok(())
    }

    async fn set_single_folder_name_by_bms(&self, work_dir: &Path) -> Result<bool, DomainError> {
        let mut info = get_dir_bms_info(&*self.fs, work_dir).await;

        while info.is_none() {
            self.output.info(&format!(
                "{} has no bms/bmson files! Trying to move out.",
                work_dir.display()
            ));

            let entries = self.fs.read_dir(work_dir).await?;
            let elements: Vec<_> = entries.into_iter().collect();

            if elements.is_empty() {
                self.output.info(" - Empty dir! Deleting...");
                match self.fs.remove_dir(work_dir).await {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        self.output.warn(&format!(" x PermissionError: {e}"));
                    }
                    Err(e) => return Err(e.into()),
                }
                return Ok(false);
            }

            if elements.len() != 1 {
                self.output
                    .info(&format!(" - Element count: {}", elements.len()));
                return Ok(false);
            }

            let inner_path = &elements[0].path;
            if !self.fs.is_dir(inner_path).await {
                self.output.info(&format!(
                    " - Folder has only a file: {:?}",
                    inner_path.file_name()
                ));
                return Ok(false);
            }

            self.output.info(" - Moving out files...");
            move_elements_across_dir(
                inner_path,
                work_dir,
                MoveOptions::default(),
                &ReplaceOptions::default(),
            )
            .await?;
            info = get_dir_bms_info(&*self.fs, work_dir).await;
        }

        let info = info.unwrap();
        let parent_dir = work_dir.parent().unwrap_or(work_dir);

        if info.title.is_empty() && info.artist.is_empty() {
            self.output.info(&format!(
                "{}: Info title and artist is EMPTY!",
                work_dir.display()
            ));
            return Ok(false);
        }

        let new_dir_path = parent_dir.join(format!(
            "{} [{}]",
            get_valid_fs_name(&info.title),
            get_valid_fs_name(&info.artist)
        ));

        if work_dir == new_dir_path {
            return Ok(true);
        }

        self.output.info(&format!(
            "{}: Rename! Title: {}; Artist: {}",
            work_dir.display(),
            info.title,
            info.artist
        ));

        if !self.fs.is_dir(&new_dir_path).await {
            self.fs.rename(work_dir, &new_dir_path).await?;
            return Ok(true);
        }

        let similarity = bms_dir_similarity(work_dir, &new_dir_path).await;
        self.output.info(&format!(
            " - Directory {} exists! Similarity: {similarity}",
            new_dir_path.display()
        ));

        if similarity < 0.8 {
            self.output.info(" - Merge canceled.");
            return Ok(false);
        }

        self.output.info(" - Merge start!");
        move_elements_across_dir(
            work_dir,
            &new_dir_path,
            MoveOptions::default(),
            &REPLACE_OPTION_UPDATE_PACK,
        )
        .await?;
        Ok(true)
    }

    /// Undo `set_name` — revert `{title} [{artist}]` dirs to bare number prefix.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if filesystem operations fail.
    pub async fn undo_set_name(&self, root_dir: &Path) -> Result<(), DomainError> {
        if !self.fs.is_dir(root_dir).await {
            return Ok(());
        }

        let entries = self.fs.read_dir(root_dir).await?;

        for entry in entries {
            if !entry.is_dir {
                continue;
            }

            let dir_name = entry.name.as_str();

            let parts: Vec<&str> = dir_name.splitn(2, ' ').collect();
            let new_dir_name = parts[0];

            if dir_name == new_dir_name {
                continue;
            }

            let new_dir_path = root_dir.join(new_dir_name);

            if self.fs.is_dir(&new_dir_path).await {
                self.output.info(&format!(
                    "Warning: Target {} already exists! Skipping {dir_name}",
                    new_dir_path.display()
                ));
                continue;
            }

            self.output
                .info(&format!("Rename {dir_name} to {new_dir_name}"));
            self.fs.rename(&entry.path, &new_dir_path).await?;
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
    async fn test_append_name_by_bms_numbers_only() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("123");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(
            work.join("test.bms"),
            "#TITLE TestSong\n#ARTIST TestArtist\n",
        )
        .await
        .unwrap();

        let service =
            FolderRenameService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.append_name_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(e) = read_dir.next_entry().await.unwrap() {
            entries.push(e);
        }
        assert_eq!(entries.len(), 1);
        let name = entries[0].file_name().to_string_lossy().to_string();
        assert!(name.contains("123. TestSong [TestArtist]"), "got: {name}");
    }

    #[tokio::test]
    async fn test_append_name_by_bms_skips_named() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("MySong");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(work.join("test.bms"), "#TITLE TestSong\n")
            .await
            .unwrap();

        let service =
            FolderRenameService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.append_name_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(e) = read_dir.next_entry().await.unwrap() {
            entries.push(e);
        }
        assert_eq!(entries.len(), 1);
        let name = entries[0].file_name().to_string_lossy().to_string();
        assert_eq!(name, "MySong", "should skip non-numeric dir: {name}");
    }

    #[tokio::test]
    async fn test_append_artist_name() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("MySong");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(work.join("test.bms"), "#TITLE Song\n#ARTIST ArtistName\n")
            .await
            .unwrap();

        let service =
            FolderRenameService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .append_artist_name_by_bms(root.path())
            .await
            .unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(e) = read_dir.next_entry().await.unwrap() {
            entries.push(e);
        }
        assert_eq!(entries.len(), 1);
        let name = entries[0].file_name().to_string_lossy().to_string();
        assert!(name.contains("[ArtistName]"), "missing artist in: {name}");
    }

    #[tokio::test]
    async fn test_append_artist_name_skips_already_set() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("MySong [Artist]");
        fs::create_dir_all(&work).await.unwrap();

        let service =
            FolderRenameService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service
            .append_artist_name_by_bms(root.path())
            .await
            .unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(e) = read_dir.next_entry().await.unwrap() {
            entries.push(e);
        }
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name().to_string_lossy(), "MySong [Artist]");
    }

    #[tokio::test]
    async fn test_set_name_by_bms_basic() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("123");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(
            work.join("test.bms"),
            "#TITLE NiceSong\n#ARTIST NiceArtist\n#GENRE NiceGenre\n",
        )
        .await
        .unwrap();

        let service =
            FolderRenameService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.set_name_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(e) = read_dir.next_entry().await.unwrap() {
            entries.push(e);
        }
        assert_eq!(entries.len(), 1);
        let name = entries[0].file_name().to_string_lossy().to_string();
        assert_eq!(name, "NiceSong [NiceArtist]");
    }

    #[tokio::test]
    async fn test_set_name_by_bms_merge() {
        let root = TempDir::new().unwrap();
        let src = root.path().join("src");
        fs::create_dir_all(&src).await.unwrap();
        fs::write(src.join("test.bms"), "#TITLE Song\n#ARTIST Artist\n")
            .await
            .unwrap();
        fs::write(src.join("a.ogg"), "audio").await.unwrap();
        fs::write(src.join("readme.txt"), "info").await.unwrap();
        let dst = root.path().join("Song [Artist]");
        fs::create_dir_all(&dst).await.unwrap();
        fs::write(dst.join("a.ogg"), "audio").await.unwrap();
        fs::write(dst.join("readme.txt"), "info").await.unwrap();
        fs::write(dst.join("b.ogg"), "audio2").await.unwrap();

        let service =
            FolderRenameService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.set_name_by_bms(root.path()).await.unwrap();

        assert!(!src.exists(), "src should be removed after merge");
        assert!(dst.join("b.ogg").is_file(), "dst extra file should survive");
        assert!(dst.join("a.ogg").is_file(), "shared file should remain");
    }

    #[tokio::test]
    async fn test_set_name_by_bms_empty_info() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("99");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(work.join("test.bms"), "#TITLE \n#ARTIST \n")
            .await
            .unwrap();

        let service =
            FolderRenameService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.set_name_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(e) = read_dir.next_entry().await.unwrap() {
            entries.push(e);
        }
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name().to_string_lossy(), "99");
    }

    #[tokio::test]
    async fn test_undo_set_name() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("NiceSong [NiceArtist]");
        fs::create_dir_all(&work).await.unwrap();

        let service =
            FolderRenameService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.undo_set_name(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(e) = read_dir.next_entry().await.unwrap() {
            entries.push(e);
        }
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name().to_string_lossy(), "NiceSong");
    }

    #[tokio::test]
    async fn test_undo_set_name_skip_conflict() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("NiceSong [Artist]");
        let conflict = root.path().join("NiceSong");
        fs::create_dir_all(&work).await.unwrap();
        fs::create_dir_all(&conflict).await.unwrap();

        let service =
            FolderRenameService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.undo_set_name(root.path()).await.unwrap();

        assert!(work.is_dir(), "original should survive");
        assert!(conflict.is_dir(), "conflict should survive");
    }
}
