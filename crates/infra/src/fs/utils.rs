use std::path::Path;
use tokio::fs;

/// Recursively copy a directory tree from `source` to `target`.
///
/// # Errors
///
/// Returns an error if `source` is not a directory or any file I/O fails.
pub async fn copy_dir_recursive(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    if !source.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            "Source is not a directory",
        ));
    }

    fs::create_dir_all(target).await?;

    let mut entries = fs::read_dir(source).await?;
    while let Some(entry) = entries.next_entry().await? {
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());

        if source_path.is_dir() {
            Box::pin(copy_dir_recursive(&source_path, &target_path)).await?;
        } else {
            fs::copy(&source_path, &target_path).await?;
        }
    }

    Ok(())
}

// External tool validation utilities.
