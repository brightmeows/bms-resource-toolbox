use std::path::Path;

use tokio::fs;

use crate::domain::port::{DirEntry, FileMetadata, FsPort};

/// Delegates all filesystem operations to `tokio::fs`.
pub struct TokioFsAdapter;

#[async_trait::async_trait]
impl FsPort for TokioFsAdapter {
    async fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>, std::io::Error> {
        let mut entries = Vec::new();
        let mut rd = fs::read_dir(path).await?;
        while let Some(entry) = rd.next_entry().await? {
            let meta = entry.metadata().await?;
            entries.push(DirEntry {
                path: entry.path(),
                name: entry.file_name().to_string_lossy().into_owned(),
                is_dir: meta.is_dir(),
                is_file: meta.is_file(),
            });
        }
        Ok(entries)
    }

    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, std::io::Error> {
        fs::read(path).await
    }

    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), std::io::Error> {
        fs::write(path, data).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<(), std::io::Error> {
        fs::rename(from, to).await
    }

    async fn remove_file(&self, path: &Path) -> Result<(), std::io::Error> {
        fs::remove_file(path).await
    }

    async fn remove_dir(&self, path: &Path) -> Result<(), std::io::Error> {
        fs::remove_dir(path).await
    }

    async fn create_dir_all(&self, path: &Path) -> Result<(), std::io::Error> {
        fs::create_dir_all(path).await
    }

    async fn metadata(&self, path: &Path) -> Result<FileMetadata, std::io::Error> {
        let meta = fs::metadata(path).await?;
        Ok(FileMetadata { len: meta.len() })
    }

    async fn is_dir(&self, path: &Path) -> bool {
        fs::symlink_metadata(path).await.is_ok_and(|m| m.is_dir())
    }

    async fn is_file(&self, path: &Path) -> bool {
        fs::symlink_metadata(path).await.is_ok_and(|m| m.is_file())
    }

    async fn exists(&self, path: &Path) -> bool {
        fs::symlink_metadata(path).await.is_ok()
    }
}
