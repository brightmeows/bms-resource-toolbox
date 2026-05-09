//! Name-based archive extraction pipeline.

use std::path::Path;
use tokio::fs;

use crate::bms::types::CHART_FILE_EXTS;
use crate::error::DomainError;
use crate::pack::flatten::move_out_files_in_folder_in_cache_dir;
use bms_res_tb_infra::archive::extract::extract_archive;
use bms_res_tb_infra::fs::utils::copy_dir_recursive;

/// Extract archives by original filename to BMS folder structure.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn unzip_with_name_to_bms_folder(
    pack_dir: &Path,
    cache_dir: &Path,
    root_dir: &Path,
) -> Result<(), DomainError> {
    tracing::info!("Unzip with name to BMS folder: {pack_dir:?} -> {root_dir:?}");

    create_directories(cache_dir, root_dir).await?;
    let archive_names = get_archive_files(pack_dir).await;

    if archive_names.is_empty() {
        tracing::info!("No archive files found in {pack_dir:?}");
        return Ok(());
    }

    for file_name in &archive_names {
        process_single_archive(pack_dir, cache_dir, root_dir, file_name).await?;
    }

    Ok(())
}

async fn create_directories(cache_dir: &Path, root_dir: &Path) -> Result<(), DomainError> {
    if !cache_dir.is_dir() {
        fs::create_dir_all(cache_dir).await?;
    }
    if !root_dir.is_dir() {
        fs::create_dir_all(root_dir).await?;
    }
    Ok(())
}

async fn get_archive_files(pack_dir: &Path) -> Vec<String> {
    let mut archive_names = Vec::new();

    if let Ok(mut read_dir) = fs::read_dir(pack_dir).await {
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            #[expect(clippy::case_sensitive_file_extension_comparisons)]
            if name.ends_with(".zip") || name.ends_with(".7z") || name.ends_with(".rar") {
                archive_names.push(name.to_string());
            }
        }
    }

    archive_names
}

async fn process_single_archive(
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
    prepare_cache_directory(&cache_dir_path).await?;

    extract_archive(&file_path, &cache_dir_path).await?;

    if !move_out_files_in_folder_in_cache_dir(&cache_dir_path, &CHART_FILE_EXTS).await {
        tracing::info!("Failed to process cache dir: {cache_dir_path:?}");
        return Ok(());
    }

    let target_dir_path = root_dir.join(&file_stem);
    move_cache_to_bms_dir(&cache_dir_path, &target_dir_path).await?;
    let _ = fs::remove_dir(&cache_dir_path).await;
    move_original_to_bofttpacks(&file_path, pack_dir, file_name).await;

    tracing::info!("Finished processing: {file_name}");
    Ok(())
}

async fn prepare_cache_directory(cache_dir_path: &Path) -> Result<(), DomainError> {
    if cache_dir_path.is_dir() {
        let has_files = {
            let mut read_dir = fs::read_dir(cache_dir_path).await?;
            loop {
                match read_dir.next_entry().await {
                    Ok(Some(entry)) => {
                        if entry.path().is_file() {
                            break true;
                        }
                    }
                    Ok(None) | Err(_) => break false,
                }
            }
        };
        if has_files {
            tracing::info!("Removing existing cache dir: {cache_dir_path:?}");
            fs::remove_dir_all(cache_dir_path).await?;
        }
    }
    fs::create_dir_all(cache_dir_path).await?;
    Ok(())
}

/// Move extracted cache files to the target BMS directory.
///
/// Falls back to copy+delete when cross-device rename fails.
pub(super) async fn move_cache_to_bms_dir(
    cache_dir_path: &Path,
    target_dir_path: &Path,
) -> Result<(), DomainError> {
    tracing::info!("Moving files from {cache_dir_path:?} to {target_dir_path:?}");

    let mut read_dir = fs::read_dir(cache_dir_path).await?;
    let mut entries = Vec::new();
    while let Some(entry) = read_dir.next_entry().await? {
        entries.push(entry);
    }

    fs::create_dir_all(target_dir_path).await?;

    for entry in entries {
        let src_path = entry.path();
        let dst_path = target_dir_path.join(src_path.file_name().unwrap_or_default());
        if fs::rename(&src_path, &dst_path).await.is_err() {
            if src_path.is_dir() {
                copy_dir_recursive(&src_path, &dst_path).await?;
                fs::remove_dir_all(&src_path).await?;
            } else {
                fs::copy(&src_path, &dst_path).await?;
                fs::remove_file(&src_path).await?;
            }
        }
    }

    Ok(())
}

async fn move_original_to_bofttpacks(file_path: &Path, pack_dir: &Path, file_name: &str) {
    let used_pack_dir = pack_dir.join("BOFTTPacks");
    if !used_pack_dir.is_dir() {
        let _ = fs::create_dir_all(&used_pack_dir).await;
    }
    let target_file_path = used_pack_dir.join(file_name);
    let _ = fs::rename(file_path, &target_file_path).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
        unzip_with_name_to_bms_folder(pack.path(), cache.path(), root.path())
            .await
            .unwrap();
    }
}
