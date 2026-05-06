use std::path::{Path, PathBuf};

/// Structured directory entry — does NOT expose `tokio::fs::DirEntry` (not mock-friendly).
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// Full path to the entry.
    pub path: PathBuf,
    /// File or directory name.
    pub name: String,
    /// Whether it is a directory.
    pub is_dir: bool,
    /// Whether it is a regular file.
    pub is_file: bool,
}

/// Structured file metadata.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FileMetadata {
    /// File size in bytes.
    pub len: u64,
}

/// File system port — domain depends on this, not `tokio::fs`.
#[async_trait::async_trait]
pub trait FsPort: Send + Sync {
    /// Read entries in a directory.
    async fn read_dir(&self, path: &Path) -> Result<Vec<DirEntry>, std::io::Error>;
    /// Read the entire contents of a file into bytes.
    async fn read_file(&self, path: &Path) -> Result<Vec<u8>, std::io::Error>;
    /// Write bytes to a file.
    async fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), std::io::Error>;
    /// Rename or move a file.
    async fn rename(&self, from: &Path, to: &Path) -> Result<(), std::io::Error>;
    /// Remove a file.
    async fn remove_file(&self, path: &Path) -> Result<(), std::io::Error>;
    /// Remove an empty directory.
    async fn remove_dir(&self, path: &Path) -> Result<(), std::io::Error>;
    /// Create a directory and all parent directories.
    async fn create_dir_all(&self, path: &Path) -> Result<(), std::io::Error>;
    /// Query file metadata.
    async fn metadata(&self, path: &Path) -> Result<FileMetadata, std::io::Error>;
    /// Check if path points to a directory.
    async fn is_dir(&self, path: &Path) -> bool;
    /// Check if path points to a regular file.
    async fn is_file(&self, path: &Path) -> bool;
    /// Check if the path exists.
    async fn exists(&self, path: &Path) -> bool;
}
