//! BMS folder cleanup operations.

use std::path::{Path, PathBuf};
use tokio::fs;

use crate::bms::types::MEDIA_FILE_EXTS;
use crate::error::DomainError;

/// Copy folder names from source to destination based on numeric prefix.
///
/// Source folders have format "num. title \[artist\]", destination folders have
/// format "num". Copies source names to destination based on numeric prefix match.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn copy_numbered_workdir_names(
    root_dir_from: &Path,
    root_dir_to: &Path,
) -> Result<(), DomainError> {
    if !root_dir_from.is_dir() || !root_dir_to.is_dir() {
        return Ok(());
    }

    let mut src_names: Vec<(String, PathBuf)> = Vec::new();
    if let Ok(mut read_dir) = fs::read_dir(root_dir_from).await {
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(String::from) else {
                continue;
            };
            src_names.push((name, path));
        }
    }

    let mut dst_read_dir = fs::read_dir(root_dir_to).await?;
    while let Some(entry) = dst_read_dir.next_entry().await? {
        let dst_path = entry.path();
        if !dst_path.is_dir() {
            continue;
        }

        let dst_name = dst_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let num_prefix = dst_name.split_whitespace().next().unwrap_or("");
        let numeric_part = num_prefix.split('.').next().unwrap_or("");

        if !numeric_part.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        for (src_name, _src_path) in &src_names {
            if src_name.starts_with(numeric_part) {
                let target_path = dst_path.with_file_name(src_name);
                if target_path != dst_path {
                    tracing::info!(
                        "Renaming {:?} -> {:?}",
                        dst_path.file_name(),
                        target_path.file_name()
                    );
                    fs::rename(&dst_path, &target_path).await?;
                }
                break;
            }
        }
    }

    Ok(())
}

/// Remove zero-sized media files and temp files.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn remove_zero_sized_media_files(
    start_dir: &Path,
    print_dir: bool,
) -> Result<(), DomainError> {
    let mut dirs_to_process = vec![start_dir.to_path_buf()];

    while let Some(current_dir) = dirs_to_process.pop() {
        if print_dir {
            tracing::info!("Entering dir: {}", current_dir.display());
        }

        if !current_dir.is_dir() {
            tracing::info!("Not a vaild dir! Aborting...");
            continue;
        }

        let mut read_dir = fs::read_dir(&current_dir).await?;
        let mut entries = Vec::new();
        while let Some(entry) = read_dir.next_entry().await? {
            entries.push(entry);
        }

        for entry in &entries {
            let element_path = entry.path();
            let element_name = element_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if element_path.is_file() {
                let is_temp_file = element_name.to_lowercase() == "desktop.ini"
                    || element_name.to_lowercase() == "thumbs.db"
                    || element_name.to_lowercase() == ".ds_store"
                    || element_name.starts_with(".trash-")
                    || element_name.starts_with("._");

                if is_temp_file {
                    match fs::remove_file(&element_path).await {
                        Ok(()) => tracing::info!(" - Remove temp file: {}", element_path.display()),
                        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                            tracing::info!(" x PermissionError!");
                        }
                        Err(_) => {}
                    }
                    continue;
                }

                if !MEDIA_FILE_EXTS
                    .iter()
                    .any(|ext| element_name.ends_with(*ext))
                {
                    continue;
                }

                match fs::metadata(&element_path).await {
                    Ok(metadata) if metadata.len() == 0 => {
                        match fs::remove_file(&element_path).await {
                            Ok(()) => {
                                tracing::info!(" - Remove empty file: {}", element_path.display());
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                                tracing::info!(" x PermissionError!");
                            }
                            Err(_) => {}
                        }
                    }
                    _ => {}
                }
            } else if element_path.is_dir() {
                dirs_to_process.push(element_path);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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

        copy_numbered_workdir_names(src.path(), dst.path())
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

        copy_numbered_workdir_names(src.path(), dst.path())
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

        remove_zero_sized_media_files(root.path(), false)
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

        remove_zero_sized_media_files(root.path(), false)
            .await
            .unwrap();

        assert!(!sub.join("empty.mp4").exists());
    }
}
