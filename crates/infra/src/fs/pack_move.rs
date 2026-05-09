use std::path::Path;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

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
