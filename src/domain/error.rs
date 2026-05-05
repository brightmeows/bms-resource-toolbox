//! Domain layer error types.

/// Domain layer unified error.
#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Archive or infra error.
    #[error("archive error: {0}")]
    Archive(#[from] anyhow::Error),
    /// User cancelled the operation.
    #[error("user cancelled")]
    Cancelled,
}
