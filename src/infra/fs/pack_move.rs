use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use tokio::fs;

/// Check whether two files have identical content by reading both into memory.
pub async fn is_same_content(file_a: &Path, file_b: &Path) -> bool {
    if !file_a.is_file() || !file_b.is_file() {
        return false;
    }
    match (fs::read(file_a).await, fs::read(file_b).await) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Options for moving or merging files between directories.
#[derive(Debug, Clone, Copy, Default)]
pub struct MoveOptions {
    /// Whether to print info about each file being moved.
    pub print_info: bool,
}

/// Action to take when a destination file already exists during a move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReplaceAction {
    /// Skip the file; do not overwrite.
    Skip = 0,
    /// Overwrite the destination file unconditionally.
    #[default]
    Replace = 1,
    /// Rename the source file with a numbered suffix if conflict.
    Rename = 2,
    /// Check content first; if different, create a numbered backup.
    CheckReplace = 12,
}

/// Per-extension replacement policy configuration for file moves.
#[derive(Debug, Clone)]
pub struct ReplaceOptions {
    /// Map of file extensions (without dot) to their replacement action.
    pub ext: HashMap<String, ReplaceAction>,
    /// Default action for extensions not present in `ext`.
    pub default: ReplaceAction,
}

impl Default for ReplaceOptions {
    fn default() -> Self {
        Self {
            ext: HashMap::new(),
            default: ReplaceAction::Replace,
        }
    }
}

/// Preset that checks BMS chart files before replacing, replacing others unconditionally.
pub static REPLACE_OPTION_UPDATE_PACK: LazyLock<ReplaceOptions> =
    LazyLock::new(|| ReplaceOptions {
        ext: HashMap::from([
            (String::from("bms"), ReplaceAction::CheckReplace),
            (String::from("bml"), ReplaceAction::CheckReplace),
            (String::from("bme"), ReplaceAction::CheckReplace),
            (String::from("pms"), ReplaceAction::CheckReplace),
            (String::from("txt"), ReplaceAction::CheckReplace),
            (String::from("bmson"), ReplaceAction::CheckReplace),
        ]),
        default: ReplaceAction::Replace,
    });

/// Default move options: no printed output.
pub const DEFAULT_MOVE_OPTIONS: MoveOptions = MoveOptions { print_info: false };

/// Default replace options: all extensions use [`ReplaceAction::Replace`] unconditionally.
pub static DEFAULT_REPLACE_OPTIONS: LazyLock<ReplaceOptions> = LazyLock::new(|| ReplaceOptions {
    ext: HashMap::new(),
    default: ReplaceAction::Replace,
});

/// Check whether a directory contains any non-empty file (recursive).
///
/// Returns `true` if any file with size > 0 is found in the directory tree.
#[must_use]
pub async fn is_dir_having_file(dir: &Path) -> bool {
    async fn check_recursive(dir: &Path) -> bool {
        let Ok(mut entries) = fs::read_dir(dir).await else {
            return false;
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file()
                && let Ok(metadata) = fs::metadata(&path).await
                && metadata.len() > 0
            {
                return true;
            } else if path.is_dir() && Box::pin(check_recursive(&path)).await {
                return true;
            }
        }

        false
    }

    if !dir.is_dir() {
        return false;
    }

    check_recursive(dir).await
}

/// Move files and subdirectories from `src` into `dst`, merging contents.
///
/// When both `src` and `dst` exist as directories, files are moved individually
/// following the given replacement policy. After moving, the source directory is
/// removed if it no longer contains files (or if the default action is not `Skip`).
///
/// # Errors
///
/// Returns an error if any file operation (read, write, rename, remove) fails.
pub async fn move_elements_across_dir(
    src: &Path,
    dst: &Path,
    options: MoveOptions,
    replace_options: &ReplaceOptions,
) -> Result<(), std::io::Error> {
    if let (Ok(src_canon), Ok(dst_canon)) =
        (fs::canonicalize(src).await, fs::canonicalize(dst).await)
        && src_canon == dst_canon
    {
        return Ok(());
    }

    if !src.is_dir() {
        return Ok(());
    }

    if !dst.is_dir() {
        return Box::pin(move_dir_as_whole(src, dst)).await;
    }

    let mut next_folder_paths: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut write_ops: Vec<(PathBuf, PathBuf)> = Vec::new();

    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let src_path = entry.path();
        let filename = entry.file_name();
        let dst_path = dst.join(&filename);

        if src_path.is_file() {
            if let Some((planned_src, planned_dst)) =
                plan_move_file(&src_path, &dst_path, replace_options).await
            {
                write_ops.push((planned_src, planned_dst));
            }
        } else if src_path.is_dir() {
            next_folder_paths.push((src_path, dst_path));
        }
    }

    for (src_path, final_dst_path) in write_ops {
        if options.print_info {
            println!("Moving {src_path:?} -> {final_dst_path:?}");
        }
        move_file(&src_path, &final_dst_path).await?;
    }

    for (src_path, dst_path) in next_folder_paths {
        Box::pin(move_elements_across_dir(
            &src_path,
            &dst_path,
            options,
            replace_options,
        ))
        .await?;
    }

    let should_clean =
        replace_options.default != ReplaceAction::Skip || !is_dir_having_file(src).await;
    if should_clean && let Err(e) = fs::remove_dir_all(src).await {
        println!("Failed to remove source directory {src:?}: {e}");
    }

    Ok(())
}

async fn plan_move_file(
    ori_path: &Path,
    dst_path: &Path,
    replace_options: &ReplaceOptions,
) -> Option<(PathBuf, PathBuf)> {
    let file_ext = ori_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    let action = replace_options
        .ext
        .get(&file_ext)
        .copied()
        .unwrap_or(replace_options.default);

    match action {
        ReplaceAction::Skip => {
            if dst_path.is_file() {
                return None;
            }
            Some((ori_path.to_path_buf(), dst_path.to_path_buf()))
        }
        ReplaceAction::Replace => Some((ori_path.to_path_buf(), dst_path.to_path_buf())),
        ReplaceAction::Rename => {
            if !dst_path.is_file() {
                return Some((ori_path.to_path_buf(), dst_path.to_path_buf()));
            }
            if is_same_content(ori_path, dst_path).await {
                return None;
            }
            for i in 0..100 {
                let stem = dst_path.file_stem().unwrap_or_default().to_string_lossy();
                let ext = dst_path
                    .extension()
                    .map(|e| e.to_string_lossy())
                    .unwrap_or_default();
                let new_name = format!("{stem}.{i}.{ext}");
                let new_dst = dst_path.with_file_name(new_name);
                if new_dst.is_file() {
                    if is_same_content(ori_path, &new_dst).await {
                        return None;
                    }
                    continue;
                }
                return Some((ori_path.to_path_buf(), new_dst));
            }
            None
        }
        ReplaceAction::CheckReplace => {
            if !dst_path.is_file() || is_same_content(ori_path, dst_path).await {
                return Some((ori_path.to_path_buf(), dst_path.to_path_buf()));
            }
            for i in 0..100 {
                let stem = dst_path.file_stem().unwrap_or_default().to_string_lossy();
                let ext = dst_path
                    .extension()
                    .map(|e| e.to_string_lossy())
                    .unwrap_or_default();
                let new_name = format!("{stem}.{i}.{ext}");
                let new_dst_path = dst_path.with_file_name(new_name);
                if new_dst_path.is_file() {
                    if is_same_content(ori_path, &new_dst_path).await {
                        return None;
                    }
                    continue;
                }
                return Some((ori_path.to_path_buf(), new_dst_path));
            }
            None
        }
    }
}

use super::utils::copy_dir_recursive;

async fn move_file(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    match fs::rename(src, dst).await {
        Ok(()) => Ok(()),
        Err(_) => {
            if src.is_dir() {
                copy_dir_recursive(src, dst).await?;
                fs::remove_dir_all(src).await
            } else {
                fs::copy(src, dst).await?;
                fs::remove_file(src).await
            }
        }
    }
}

async fn move_dir_as_whole(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    if fs::rename(src, dst).await.is_err() {
        copy_dir_recursive(src, dst).await?;
        set_mtime_recursive(src, dst).await?;
        fs::remove_dir_all(src).await
    } else {
        Ok(())
    }
}

async fn set_mtime_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    let src_meta = fs::metadata(src).await?;
    let mtime = filetime::FileTime::from_last_modification_time(&src_meta);
    filetime::set_file_mtime(dst, mtime).map_err(std::io::Error::other)?;

    if src_meta.is_dir() {
        let mut entries = fs::read_dir(src).await?;
        while let Some(entry) = entries.next_entry().await? {
            let src_child = entry.path();
            let dst_child = dst.join(entry.file_name());
            Box::pin(set_mtime_recursive(&src_child, &dst_child)).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_replace_action_rename_no_conflict() {
        let dir = TempDir::new().unwrap();
        let src_file = dir.path().join("test.txt");
        let dst_dir = TempDir::new().unwrap();
        let dst_file = dst_dir.path().join("test.txt");
        fs::write(&src_file, "content").await.unwrap();

        let opts = ReplaceOptions {
            ext: HashMap::new(),
            default: ReplaceAction::Rename,
        };
        let result = plan_move_file(&src_file, &dst_file, &opts).await;
        assert!(result.is_some());
        let (_, planned_dst) = result.unwrap();
        assert_eq!(planned_dst, dst_file);
    }

    #[tokio::test]
    async fn test_replace_action_rename_numbered() {
        let dir = TempDir::new().unwrap();
        let src_file = dir.path().join("test.txt");
        let dst_dir = TempDir::new().unwrap();
        let dst_file = dst_dir.path().join("test.txt");
        fs::write(&src_file, "content2").await.unwrap();
        fs::write(&dst_file, "content1").await.unwrap();

        let opts = ReplaceOptions {
            ext: HashMap::new(),
            default: ReplaceAction::Rename,
        };
        let result = plan_move_file(&src_file, &dst_file, &opts).await;
        assert!(result.is_some());
        let (_, planned_dst) = result.unwrap();
        let expected = dst_file.with_file_name("test.0.txt");
        assert_eq!(planned_dst, expected);
    }

    #[tokio::test]
    async fn test_replace_action_rename_same_content_skip() {
        let dir = TempDir::new().unwrap();
        let src_file = dir.path().join("test.txt");
        let dst_dir = TempDir::new().unwrap();
        let dst_file = dst_dir.path().join("test.txt");
        fs::write(&src_file, "same").await.unwrap();
        fs::write(&dst_file, "same").await.unwrap();

        let opts = ReplaceOptions {
            ext: HashMap::new(),
            default: ReplaceAction::Rename,
        };
        let result = plan_move_file(&src_file, &dst_file, &opts).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_replace_action_rename_exhausted() {
        let dir = TempDir::new().unwrap();
        let src_file = dir.path().join("test.txt");
        let dst_dir = TempDir::new().unwrap();
        let dst_file = dst_dir.path().join("test.txt");
        fs::write(&src_file, "unique").await.unwrap();
        fs::write(&dst_file, "content1").await.unwrap();
        for i in 0..100 {
            fs::write(
                dst_dir.path().join(format!("test.{i}.txt")),
                format!("other{i}"),
            )
            .await
            .unwrap();
        }

        let opts = ReplaceOptions {
            ext: HashMap::new(),
            default: ReplaceAction::Rename,
        };
        let result = plan_move_file(&src_file, &dst_file, &opts).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_move_to_non_existent_dest_as_whole() {
        let src = TempDir::new().unwrap();
        let non_exist = TempDir::new().unwrap();
        let dst = non_exist.path().join("moved_whole");
        let _ = fs::remove_dir_all(&dst).await;
        fs::write(src.path().join("a.txt"), "data").await.unwrap();
        fs::write(src.path().join("b.txt"), "data").await.unwrap();

        let opts = MoveOptions::default();
        let rep = ReplaceOptions::default();
        move_elements_across_dir(src.path(), &dst, opts, &rep)
            .await
            .unwrap();

        assert!(!src.path().exists());
        assert!(dst.is_dir());
        assert!(dst.join("a.txt").is_file());
        assert!(dst.join("b.txt").is_file());
    }

    #[tokio::test]
    async fn test_is_dir_having_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, b"test content").await.unwrap();

        assert!(is_dir_having_file(dir.path()).await);

        assert!(!is_dir_having_file(&PathBuf::from("/nonexistent")).await);

        let empty_dir = TempDir::new().unwrap();
        assert!(!is_dir_having_file(empty_dir.path()).await);
    }
}
