//! Numeric-prefixed archive extraction pipeline.

use std::path::{Path, PathBuf};
use tokio::fs;

use crate::bms::types::CHART_FILE_EXTS;
use crate::error::DomainError;
use crate::pack::flatten::{get_num_set_file_names, move_out_files_in_folder_in_cache_dir};
use bms_res_tb_infra::archive::extract::extract_archive;
use bms_res_tb_infra::fs::pack_move::is_dir_having_file;

/// Extract numeric-prefixed archives to BMS folder structure.
///
/// Gets numbered file list (e.g., "001 filename.zip"), extracts to `cache_dir/{id}`
/// for each file, finds or creates target directory in `root_dir` with exact numeric
/// match, moves extracted files to target directory, moves original archive to
/// `BOFTTPacks/`.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn unzip_numeric_to_bms_folder(
    pack_dir: &Path,
    cache_dir: &Path,
    root_dir: &Path,
) -> Result<(), DomainError> {
    tracing::info!("Unzip numeric to BMS folder: {pack_dir:?} -> {root_dir:?}");

    if !cache_dir.is_dir() {
        fs::create_dir_all(cache_dir).await?;
    }
    if !root_dir.is_dir() {
        fs::create_dir_all(root_dir).await?;
    }

    let num_set_file_names = get_num_set_file_names(pack_dir).await;
    tracing::info!("Found {} numbered pack files", num_set_file_names.len());

    for file_name in &num_set_file_names {
        let file_path = pack_dir.join(file_name);
        if !file_path.is_file() {
            continue;
        }

        let id_str = file_name.split_whitespace().next().unwrap_or("");
        if id_str.is_empty() {
            continue;
        }

        let cache_dir_path = cache_dir.join(id_str);

        if cache_dir_path.is_dir() && is_dir_having_file(&cache_dir_path).await {
            fs::remove_dir_all(&cache_dir_path).await?;
        }
        if !cache_dir_path.is_dir() {
            fs::create_dir_all(&cache_dir_path).await?;
        }

        tracing::info!("Extracting {file_path:?} to {cache_dir_path:?}");
        extract_archive(&file_path, &cache_dir_path).await?;

        if !move_out_files_in_folder_in_cache_dir(&cache_dir_path, &CHART_FILE_EXTS).await {
            tracing::info!("Failed to process cache dir: {cache_dir_path:?}");
            continue;
        }

        let mut target_dir_path: Option<PathBuf> = None;

        if let Ok(mut read_dir) = fs::read_dir(root_dir).await {
            while let Some(entry) = read_dir.next_entry().await? {
                let dir_path = entry.path();
                if !dir_path.is_dir() {
                    continue;
                }
                let dir_name = dir_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

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
                target_dir_path = Some(dir_path);
                break;
            }
        }

        let target_dir_path = target_dir_path.unwrap_or_else(|| root_dir.join(id_str));

        super::unzip_name::move_cache_to_bms_dir(&cache_dir_path, &target_dir_path).await?;
        let _ = fs::remove_dir(&cache_dir_path).await;

        tracing::info!("Finished processing: {file_name}");
        let used_pack_dir = pack_dir.join("BOFTTPacks");
        if !used_pack_dir.is_dir() {
            fs::create_dir_all(&used_pack_dir).await?;
        }
        let target_file_path = used_pack_dir.join(file_name);
        fs::rename(&file_path, &target_file_path).await.ok();
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
pub async fn set_file_num(dir: &Path, file_idx: usize, num: i32) -> Result<(), DomainError> {
    const ALLOWED_EXTS: &[&str] = &["zip", "7z", "rar", "mp4", "bms", "bme", "bml", "pms"];

    tracing::info!("Setting file numbers in: {dir:?}");

    let mut file_names: Vec<String> = Vec::new();

    if let Ok(mut read_dir) = fs::read_dir(dir).await {
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if name
                .split_whitespace()
                .next()
                .is_some_and(|s| s.chars().all(|c| c.is_ascii_digit()))
            {
                continue;
            }

            let part_file_path = path.with_file_name(format!("{name}.part"));
            if part_file_path.is_file() {
                continue;
            }

            if fs::metadata(&path).await.map_or(true, |m| m.len() == 0) {
                continue;
            }

            let ext = name.rsplit('.').next().unwrap_or("");
            if !ALLOWED_EXTS.contains(&ext) {
                continue;
            }

            file_names.push(name.to_string());
        }
    }

    if file_names.is_empty() {
        tracing::info!("No files to number");
        return Ok(());
    }

    tracing::info!("Here are files in {}:", dir.display());
    for (i, name) in file_names.iter().enumerate() {
        tracing::info!(" - {i}: {name}");
    }

    let Some(file_name) = file_names.get(file_idx) else {
        tracing::warn!(
            "Invalid file index {file_idx}, max is {}",
            file_names.len() - 1
        );
        return Ok(());
    };

    let file_path = dir.join(file_name);
    let new_file_name = format!("{num} {file_name}");
    let new_file_path = dir.join(&new_file_name);

    tracing::info!("Rename {file_name} to {new_file_name}");
    fs::rename(&file_path, &new_file_path).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_set_file_num_basic() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("song.bms"), "#TITLE Song\n")
            .await
            .unwrap();
        set_file_num(dir.path(), 0, 42).await.unwrap();
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
        set_file_num(dir.path(), 0, 99).await.unwrap();
        assert!(
            dir.path().join("10 song.bms").is_file(),
            "already numbered should be skipped"
        );
    }

    #[tokio::test]
    async fn test_set_file_num_skips_empty_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("song.bms"), "").await.unwrap();
        set_file_num(dir.path(), 0, 1).await.unwrap();
        assert!(dir.path().join("song.bms").is_file());
    }
}
