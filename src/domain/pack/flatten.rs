use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

use crate::domain::folder::pack_move::{
    DEFAULT_MOVE_OPTIONS, DEFAULT_REPLACE_OPTIONS, move_elements_across_dir,
};

/// Get filenames in a directory that start with a numeric ID (ASCII or fullwidth digits).
///
/// These are typically set file names in a BMS cache directory.
#[must_use]
pub async fn get_num_set_file_names(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();

    let Ok(mut entries) = fs::read_dir(dir).await else {
        return names;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        if !entry.path().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let id_str = name.split(' ').next().unwrap_or("");
        if id_str.is_empty()
            || !id_str
                .chars()
                .all(|c| c.is_ascii_digit() || ('\u{FF10}'..='\u{FF19}').contains(&c))
        {
            continue;
        }
        names.push(name);
    }
    names
}

/// Flatten a cache directory by moving inner files up one level.
///
/// Handles `__MACOSX` cleanup, nested subdirectories, and inner inner directories.
/// Returns `true` if the cache was successfully processed, `false` if the cache is
/// empty or could not be fully processed automatically.
pub async fn move_out_files_in_folder_in_cache_dir(
    cache_dir_path: &Path,
    chart_exts: &[&str],
) -> bool {
    let mut error = false;
    let mut file_ext_count: HashMap<String, Vec<String>>;
    loop {
        file_ext_count = HashMap::new();
        let mut cache_folder_count: usize = 0;
        let mut cache_file_count: usize = 0;
        let mut inner_dir_name: Option<String> = None;

        let Ok(mut entries) = fs::read_dir(cache_dir_path).await else {
            break;
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let cache_path = entry.path();
            let cache_name = cache_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if cache_path.is_dir() {
                if cache_name == "__MACOSX" {
                    println!("Removing __MACOSX directory: {cache_path:?}");
                    if let Err(e) = fs::remove_dir_all(&cache_path).await {
                        println!("Failed to remove __MACOSX: {e}");
                    }
                    continue;
                }
                cache_folder_count += 1;
                inner_dir_name = Some(cache_name.to_string());
            }

            if cache_path.is_file() {
                cache_file_count += 1;
                let ext = crate::infra::fs::utils::get_ext(&cache_path).to_string();
                file_ext_count
                    .entry(ext)
                    .or_default()
                    .push(cache_name.to_string());
            }
        }

        let done;
        if cache_folder_count == 0 || (cache_folder_count == 1 && cache_file_count >= 10) {
            done = true;
        } else if cache_folder_count > 1 {
            let has_bms = has_file_with_ext_recursive(cache_dir_path, chart_exts).await;
            if has_bms {
                done = true;
            } else {
                println!(
                    " !_! {}: has more than 1 folders, please do it manually.",
                    cache_dir_path.display()
                );
                error = true;
                done = false;
            }
        } else {
            done = false;
        }

        if done || error {
            break;
        }

        if let Some(ref inner_name) = inner_dir_name
            && move_inner_dir(cache_dir_path, inner_name).await
        {
            error = true;
            break;
        }
    }

    let (final_folder_count, final_file_count) = count_cache_contents(cache_dir_path).await;

    if error {
        return false;
    }

    if final_folder_count == 0 && final_file_count == 0 {
        println!(" !_! {}: Cache is Empty!", cache_dir_path.display());
        let _ = fs::remove_dir(cache_dir_path).await;
        return false;
    }

    let mp4_count = file_ext_count.get("mp4").map_or(0, Vec::len);
    if mp4_count > 1 {
        println!(
            " - Tips: {} has more than 1 mp4 files!",
            cache_dir_path.display()
        );
    }

    true
}

async fn has_file_with_ext_recursive(dir: &Path, exts: &[&str]) -> bool {
    let Ok(mut entries) = fs::read_dir(dir).await else {
        return false;
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_file()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && exts.iter().any(|ext| name.to_lowercase().ends_with(ext))
        {
            return true;
        } else if path.is_dir() && Box::pin(has_file_with_ext_recursive(&path, exts)).await {
            return true;
        }
    }
    false
}

/// Move files from inner dir up to cache root. Returns `true` on error.
async fn move_inner_dir(cache_dir_path: &Path, inner_name: &str) -> bool {
    let inner_dir_path = cache_dir_path.join(inner_name);
    let inner_inner_dir_path = inner_dir_path.join(inner_name);
    if inner_inner_dir_path.is_dir() {
        println!(" - Renaming inner inner dir name: {inner_inner_dir_path:?}");
        let new_path = inner_inner_dir_path.with_file_name(format!("{inner_name}-rep"));
        if let Err(e) = fs::rename(&inner_inner_dir_path, &new_path).await {
            println!("Failed to rename inner inner dir: {e}");
        }
    }
    println!(" - Moving inner files in {inner_dir_path:?} to {cache_dir_path:?}");
    if let Err(e) = move_elements_across_dir(
        &inner_dir_path,
        cache_dir_path,
        DEFAULT_MOVE_OPTIONS,
        &DEFAULT_REPLACE_OPTIONS,
    )
    .await
    {
        println!("Failed to move elements: {e}");
        return true;
    }
    let _ = fs::remove_dir(&inner_dir_path).await;
    false
}

async fn count_cache_contents(dir: &Path) -> (usize, usize) {
    let mut folder_count = 0;
    let mut file_count = 0;

    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if entry.path().is_dir() {
                folder_count += 1;
            } else if entry.path().is_file() {
                file_count += 1;
            }
        }
    }

    (folder_count, file_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_flatten_single_nested() {
        let cache = TempDir::new().unwrap();
        let inner = cache.path().join("inner");
        fs::create_dir_all(&inner).await.unwrap();
        fs::write(inner.join("song.wav"), "data").await.unwrap();
        assert!(move_out_files_in_folder_in_cache_dir(cache.path(), &[".bms"]).await);
        assert!(cache.path().join("song.wav").is_file());
        assert!(!inner.exists());
    }

    #[tokio::test]
    async fn test_flatten_multi_level() {
        let cache = TempDir::new().unwrap();
        let deep = cache.path().join("a").join("b").join("c");
        fs::create_dir_all(&deep).await.unwrap();
        fs::write(deep.join("song.wav"), "data").await.unwrap();
        assert!(move_out_files_in_folder_in_cache_dir(cache.path(), &[".bms"]).await);
        assert!(cache.path().join("song.wav").is_file());
    }

    #[tokio::test]
    async fn test_flatten_macosx_removed() {
        let cache = TempDir::new().unwrap();
        fs::create_dir_all(cache.path().join("__MACOSX"))
            .await
            .unwrap();
        fs::write(cache.path().join("song.wav"), "data")
            .await
            .unwrap();
        assert!(move_out_files_in_folder_in_cache_dir(cache.path(), &[".bms"]).await);
        assert!(!cache.path().join("__MACOSX").exists());
    }

    #[tokio::test]
    async fn test_flatten_empty() {
        let cache = TempDir::new().unwrap();
        let path = cache.path().to_path_buf();
        assert!(!move_out_files_in_folder_in_cache_dir(&path, &[".bms"]).await);
        // The function removes the cache dir when empty; manual cleanup not needed.
    }

    #[tokio::test]
    async fn test_flatten_already_flat() {
        let cache = TempDir::new().unwrap();
        fs::write(cache.path().join("a.bms"), "data").await.unwrap();
        assert!(move_out_files_in_folder_in_cache_dir(cache.path(), &[".bms"]).await);
    }

    #[tokio::test]
    async fn test_get_num_set_file_names() {
        let dir = TempDir::new().unwrap();
        let _names = get_num_set_file_names(dir.path()).await;
    }
}
