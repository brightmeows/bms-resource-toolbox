//! Path validation functions for interactive command parameters.
//!
//! Provides semantic checks like `is_root_dir`, `is_work_dir`, etc.

use std::path::Path;

/// Chart file extensions used for semantic validation.
const CHART_EXTS: &[&str] = &["bms", "bme", "bml", "pms", "bmson"];

/// Check if a path exists and is a directory.
#[expect(dead_code, reason = "Public API for future validation use")]
#[must_use]
pub fn exists_as_dir(path: &Path) -> bool {
    path.is_dir()
}

/// Check if a path exists as a file.
#[expect(dead_code, reason = "Public API for future validation use")]
#[must_use]
pub fn exists_as_file(path: &Path) -> bool {
    path.is_file()
}

/// Check that the directory does NOT directly contain chart files.
///
/// A "root dir" should contain work subdirectories, each with their own chart files.
#[must_use]
pub fn is_root_dir(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    let Ok(mut entries) = std::fs::read_dir(path) else {
        return false;
    };

    while let Some(Ok(entry)) = entries.next() {
        if !entry.path().is_file() {
            continue;
        }
        let fname = entry.file_name();
        let Some(name) = fname.to_str() else {
            continue;
        };
        let lower = name.to_lowercase();
        if CHART_EXTS.iter().any(|ext| lower.ends_with(ext)) {
            return false;
        }
    }

    true
}

/// Check that the directory directly contains at least one chart file.
#[must_use]
pub fn is_work_dir(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    let Ok(mut entries) = std::fs::read_dir(path) else {
        return false;
    };

    while let Some(Ok(entry)) = entries.next() {
        if !entry.path().is_file() {
            continue;
        }
        let fname = entry.file_name();
        let Some(name) = fname.to_str() else {
            continue;
        };
        let lower = name.to_lowercase();
        if CHART_EXTS.iter().any(|ext| lower.ends_with(ext)) {
            return true;
        }
    }

    false
}

/// Validate a path against the given semantic constraint.
///
/// Returns `Ok(())` if valid, or `Err` with a user-facing error message.
pub fn validate_path(path: &Path, semantic: super::types::PathSemantic) -> Result<(), String> {
    use super::types::PathSemantic;

    match semantic {
        PathSemantic::Any => {
            if !path.is_dir() {
                return Err(format!("路径不是有效的目录: {}", path.display()));
            }
            Ok(())
        }
        PathSemantic::RootDir => {
            if !path.is_dir() {
                return Err(format!("目录不存在: {}", path.display()));
            }
            if !is_root_dir(path) {
                return Err(format!(
                    "目录 {} 包含谱面文件 (期望是根目录，不应直接包含 .bms/.bme 等文件)",
                    path.display()
                ));
            }
            Ok(())
        }
        PathSemantic::WorkDir => {
            if !path.is_dir() {
                return Err(format!("目录不存在: {}", path.display()));
            }
            if !is_work_dir(path) {
                return Err(format!(
                    "目录 {} 不包含任何谱面文件",
                    path.display()
                ));
            }
            Ok(())
        }
        PathSemantic::NonExistent => {
            if path.exists() {
                return Err(format!("路径已存在: {}", path.display()));
            }
            Ok(())
        }
        PathSemantic::File => {
            if !path.is_file() {
                return Err(format!("文件不存在: {}", path.display()));
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_is_root_dir_valid() {
        let tmp = TempDir::new().unwrap();
        let work = tmp.path().join("work1");
        std::fs::create_dir_all(&work).unwrap();
        // root dir has subdirectories, not chart files directly
        assert!(is_root_dir(tmp.path()));
    }

    #[test]
    fn test_is_root_dir_invalid_has_charts() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("song.bms"), "#TITLE Song").unwrap();
        assert!(!is_root_dir(tmp.path()));
    }

    #[test]
    fn test_is_work_dir_valid() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("song.bms"), "#TITLE Song").unwrap();
        assert!(is_work_dir(tmp.path()));
    }

    #[test]
    fn test_is_work_dir_invalid_no_charts() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("readme.txt"), "info").unwrap();
        assert!(!is_work_dir(tmp.path()));
    }

    #[test]
    fn test_validate_root_dir() {
        use super::super::types::PathSemantic;

        let tmp = TempDir::new().unwrap();
        assert!(validate_path(tmp.path(), PathSemantic::RootDir).is_ok());

        std::fs::write(tmp.path().join("song.bms"), "#TITLE Song").unwrap();
        assert!(validate_path(tmp.path(), PathSemantic::RootDir).is_err());
    }

    #[test]
    fn test_validate_non_existent() {
        use super::super::types::PathSemantic;

        let tmp = TempDir::new().unwrap();
        let non_existent = tmp.path().join("does_not_exist");
        assert!(validate_path(&non_existent, PathSemantic::NonExistent).is_ok());
        assert!(validate_path(tmp.path(), PathSemantic::NonExistent).is_err());
    }
}
