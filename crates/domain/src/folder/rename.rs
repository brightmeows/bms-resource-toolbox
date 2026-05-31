//! BMS folder rename operations.

use std::path::{Path, PathBuf};
use tokio::fs;

use crate::bms::dir::get_dir_bms_info;
use crate::error::DomainError;
use crate::folder::pack_move::{
    MoveOptions, REPLACE_OPTION_UPDATE_PACK, ReplaceOptions, move_elements_across_dir,
};
use crate::folder::similarity::bms_dir_similarity;
use crate::util::get_valid_fs_name;

/// Target component(s) for set-name operations.
#[derive(Clone, Copy)]
pub enum SetNameTarget {
    /// Set to "title \[artist\]"
    All,
    /// Set to just the title
    Title,
    /// Set to just the artist
    Artist,
}

/// Target component(s) for append operations.
#[derive(Clone, Copy)]
pub enum AppendTarget {
    /// Append "title \[artist\]" to numeric folder names
    All,
    /// Append just the title
    Title,
    /// Append "\[artist\]" to folder name
    Artist,
}

// ── append helpers ──────────────────────────────────────

/// Append title and artist info to folder names based on BMS files.
///
/// Iterates through subdirectories, renames folders that are purely numeric
/// to "num. title \[artist\]" format.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn append_name_by_bms(root_dir: &Path) -> Result<(), DomainError> {
    append_by_target(root_dir, AppendTarget::All).await
}

/// Append title info to folder names based on BMS files.
///
/// Adds " \[title\]" suffix to folders.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn append_title_by_bms(root_dir: &Path) -> Result<(), DomainError> {
    append_by_target(root_dir, AppendTarget::Title).await
}

/// Append artist name to folder names based on BMS files.
///
/// Adds " \[artist\]" suffix to folders not already ending with "\]".
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn append_artist_name_by_bms(root_dir: &Path) -> Result<(), DomainError> {
    append_by_target(root_dir, AppendTarget::Artist).await
}

async fn append_by_target(root_dir: &Path, target: AppendTarget) -> Result<(), DomainError> {
    if !root_dir.is_dir() {
        return Ok(());
    }

    let mut pairs: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut read_dir = fs::read_dir(root_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let dir_path = entry.path();
        if !dir_path.is_dir() {
            continue;
        }

        let dir_name = dir_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        match target {
            AppendTarget::All => {
                if !dir_name.trim().is_empty()
                    && dir_name
                        .chars()
                        .all(|c| c.is_ascii_digit() || ('\u{FF10}'..='\u{FF19}').contains(&c))
                    && let Some(new_name) = rename_folder_by_bms(&dir_path).await
                {
                    let new_path = dir_path.with_file_name(&new_name);
                    pairs.push((dir_path, new_path));
                }
            }
            AppendTarget::Artist => {
                if dir_name.ends_with(']') {
                    continue;
                }

                let info = get_dir_bms_info(&dir_path).await;
                let Some(info) = info else {
                    tracing::info!("Dir {} has no bms files!", dir_path.display());
                    continue;
                };

                let new_dir_name = format!("{dir_name} [{}]", get_valid_fs_name(&info.artist));
                tracing::info!("- Ready to rename: {dir_name} -> {new_dir_name}");
                pairs.push((dir_path, root_dir.join(&new_dir_name)));
            }
            AppendTarget::Title => {
                if dir_name.ends_with(']') {
                    continue;
                }

                let info = get_dir_bms_info(&dir_path).await;
                let Some(info) = info else {
                    tracing::info!("Dir {} has no bms files!", dir_path.display());
                    continue;
                };

                let new_dir_name = format!("{} {}", dir_name, get_valid_fs_name(&info.title));
                tracing::info!("- Ready to rename: {dir_name} -> {new_dir_name}");
                pairs.push((dir_path, root_dir.join(&new_dir_name)));
            }
        }
    }

    if pairs.is_empty() {
        tracing::info!("No folders to rename");
        return Ok(());
    }

    for (from, to) in pairs {
        fs::rename(&from, &to).await?;
    }

    Ok(())
}

/// Rename a single folder based on its BMS info
///
/// Returns the new folder name if renamed, `None` if skipped.
#[must_use]
async fn rename_folder_by_bms(work_dir: &Path) -> Option<String> {
    let dir_name = work_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if !dir_name.trim().is_empty()
        && !dir_name
            .chars()
            .all(|c| c.is_ascii_digit() || ('\u{FF10}'..='\u{FF19}').contains(&c))
    {
        return None;
    }

    let info = get_dir_bms_info(work_dir).await?;
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

// ── set-name helpers ────────────────────────────────────

/// Set folder names based on BMS info (title \[artist\] format).
///
/// Renames folders to "title \[artist\]" format, handles merging if target
/// already exists (with similarity check).
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn set_name_by_bms(root_dir: &Path) -> Result<(), DomainError> {
    set_name_by_target(root_dir, SetNameTarget::All).await
}

/// Set folder names to just the BMS title.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn set_title_by_bms(root_dir: &Path) -> Result<(), DomainError> {
    set_name_by_target(root_dir, SetNameTarget::Title).await
}

/// Set folder names to just the BMS artist.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn set_artist_by_bms(root_dir: &Path) -> Result<(), DomainError> {
    set_name_by_target(root_dir, SetNameTarget::Artist).await
}

async fn set_name_by_target(root_dir: &Path, target: SetNameTarget) -> Result<(), DomainError> {
    if !root_dir.is_dir() {
        return Ok(());
    }

    let mut fail_list: Vec<String> = Vec::new();

    let mut read_dir = fs::read_dir(root_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let dir_path = entry.path();
        if !dir_path.is_dir() {
            continue;
        }

        let dir_name = dir_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if !set_single_folder_name_by_bms(&dir_path, target).await? {
            fail_list.push(dir_name);
        }
    }

    if !fail_list.is_empty() {
        tracing::info!("Fail Count: {}", fail_list.len());
        for name in &fail_list {
            tracing::info!("  {name}");
        }
    }

    Ok(())
}

/// Set a single folder's name based on its BMS info and target.
/// Returns true if successful, false if skipped or failed.
async fn set_single_folder_name_by_bms(
    work_dir: &Path,
    target: SetNameTarget,
) -> Result<bool, DomainError> {
    let mut info = get_dir_bms_info(work_dir).await;

    while info.is_none() {
        tracing::info!(
            "{} has no bms/bmson files! Trying to move out.",
            work_dir.display()
        );

        let mut elements: Vec<_> = Vec::new();
        let mut read_dir = fs::read_dir(work_dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            elements.push(entry);
        }

        if elements.is_empty() {
            tracing::info!(" - Empty dir! Deleting...");
            match fs::remove_dir(work_dir).await {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                    tracing::info!(" x PermissionError: {e}");
                }
                Err(e) => return Err(e.into()),
            }
            return Ok(false);
        }

        if elements.len() != 1 {
            tracing::info!(" - Element count: {}", elements.len());
            return Ok(false);
        }

        let inner_path = elements.first().expect("len == 1").path();
        if !inner_path.is_dir() {
            tracing::info!(" - Folder has only a file: {:?}", inner_path.file_name());
            return Ok(false);
        }

        tracing::info!(" - Moving out files...");
        move_elements_across_dir(
            &inner_path,
            work_dir,
            MoveOptions::default(),
            &ReplaceOptions::default(),
        )
        .await?;
        info = get_dir_bms_info(work_dir).await;
    }

    let info = info.expect("while loop ensured info is Some");
    let parent_dir = work_dir.parent().unwrap_or(work_dir);

    if info.title.is_empty() && info.artist.is_empty() {
        tracing::info!("{}: Info title and artist is EMPTY!", work_dir.display());
        return Ok(false);
    }

    let new_name = format_set_name(&info.title, &info.artist, target);
    let new_dir_path = parent_dir.join(new_name);

    if work_dir == new_dir_path {
        return Ok(true);
    }

    tracing::info!(
        "{}: Rename! Title: {}; Artist: {}",
        work_dir.display(),
        info.title,
        info.artist
    );

    if !new_dir_path.is_dir() {
        fs::rename(work_dir, &new_dir_path).await?;
        return Ok(true);
    }

    let similarity = bms_dir_similarity(work_dir, &new_dir_path).await;
    tracing::info!(
        " - Directory {} exists! Similarity: {similarity}",
        new_dir_path.display()
    );

    if similarity < 0.8 {
        tracing::info!(" - Merge canceled.");
        return Ok(false);
    }

    tracing::info!(" - Merge start!");
    move_elements_across_dir(
        work_dir,
        &new_dir_path,
        MoveOptions::default(),
        &REPLACE_OPTION_UPDATE_PACK,
    )
    .await?;
    Ok(true)
}

/// Format the new folder name based on the target.
fn format_set_name(title: &str, artist: &str, target: SetNameTarget) -> String {
    match target {
        SetNameTarget::All => format!(
            "{} [{}]",
            get_valid_fs_name(title),
            get_valid_fs_name(artist)
        ),
        SetNameTarget::Title => get_valid_fs_name(title),
        SetNameTarget::Artist => get_valid_fs_name(artist),
    }
}

/// Undo `set_name` by removing " \[artist\]" suffix.
///
/// Removes " \[artist\]" part from folder names and restores the original numeric prefix.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn undo_set_name(root_dir: &Path) -> Result<(), DomainError> {
    if !root_dir.is_dir() {
        return Ok(());
    }

    let mut dir_entries: Vec<fs::DirEntry> = Vec::new();
    let mut read_dir = fs::read_dir(root_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        dir_entries.push(entry);
    }

    for entry in dir_entries {
        let dir_path = entry.path();
        if !dir_path.is_dir() {
            continue;
        }

        let dir_name = dir_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let parts: Vec<&str> = dir_name.splitn(2, ' ').collect();
        let new_dir_name = parts.first().copied().unwrap_or(dir_name);

        if dir_name == new_dir_name {
            continue;
        }

        let new_dir_path = root_dir.join(new_dir_name);

        if new_dir_path.is_dir() {
            tracing::warn!(
                "Warning: Target {} already exists! Skipping {dir_name}",
                new_dir_path.display()
            );
            continue;
        }

        tracing::info!("Rename {dir_name} to {new_dir_name}");
        fs::rename(&dir_path, &new_dir_path).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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

        append_name_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.unwrap() {
            entries.push(entry);
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

        append_name_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.unwrap() {
            entries.push(entry);
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

        append_artist_name_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.unwrap() {
            entries.push(entry);
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

        append_artist_name_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.unwrap() {
            entries.push(entry);
        }
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name().to_string_lossy(), "MySong [Artist]");
    }

    #[tokio::test]
    async fn test_append_title() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("MySong");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(
            work.join("test.bms"),
            "#TITLE GreatTitle\n#ARTIST ArtistName\n",
        )
        .await
        .unwrap();

        append_title_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.unwrap() {
            entries.push(entry);
        }
        assert_eq!(entries.len(), 1);
        let name = entries[0].file_name().to_string_lossy().to_string();
        assert!(name.contains("GreatTitle"), "missing title in: {name}");
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

        set_name_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.unwrap() {
            entries.push(entry);
        }
        assert_eq!(entries.len(), 1);
        let name = entries[0].file_name().to_string_lossy().to_string();
        assert_eq!(name, "NiceSong [NiceArtist]");
    }

    #[tokio::test]
    async fn test_set_title_by_bms() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("123");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(
            work.join("test.bms"),
            "#TITLE NiceSong\n#ARTIST NiceArtist\n",
        )
        .await
        .unwrap();

        set_title_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.unwrap() {
            entries.push(entry);
        }
        assert_eq!(entries.len(), 1);
        let name = entries[0].file_name().to_string_lossy().to_string();
        assert_eq!(name, "NiceSong");
    }

    #[tokio::test]
    async fn test_set_artist_by_bms() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("123");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(
            work.join("test.bms"),
            "#TITLE NiceSong\n#ARTIST NiceArtist\n",
        )
        .await
        .unwrap();

        set_artist_by_bms(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.unwrap() {
            entries.push(entry);
        }
        assert_eq!(entries.len(), 1);
        let name = entries[0].file_name().to_string_lossy().to_string();
        assert_eq!(name, "NiceArtist");
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

        set_name_by_bms(root.path()).await.unwrap();

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

        set_name_by_bms(root.path()).await.unwrap();
        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.unwrap() {
            entries.push(entry);
        }
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name().to_string_lossy(), "99");
    }

    #[tokio::test]
    async fn test_undo_set_name() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("NiceSong [NiceArtist]");
        fs::create_dir_all(&work).await.unwrap();

        undo_set_name(root.path()).await.unwrap();

        let mut read_dir = fs::read_dir(root.path()).await.unwrap();
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await.unwrap() {
            entries.push(entry);
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

        undo_set_name(root.path()).await.unwrap();

        assert!(work.is_dir(), "original should survive");
        assert!(conflict.is_dir(), "conflict should survive");
    }
}
