//! Port traits for hexagonal architecture — domain-layer abstractions over infra.

/// System browser abstraction.
pub mod browser;
/// File system abstraction.
pub mod fs;
/// User output (info/warn/error) abstraction.
pub mod output;
/// XLSX workbook creation abstraction.
pub mod xlsx;

pub use browser::BrowserPort;
pub use fs::{DirEntry, FileMetadata, FsPort};
pub use output::OutputPort;
pub use xlsx::{XlsxPort, XlsxWorkbook};
