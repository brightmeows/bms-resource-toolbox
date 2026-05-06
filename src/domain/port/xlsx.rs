use std::path::Path;

use crate::domain::error::DomainError;

/// Port for creating XLSX workbooks.
pub trait XlsxPort: Send + Sync {
    /// Create a new workbook.
    fn create_workbook(&self) -> Box<dyn XlsxWorkbook>;
}

/// A writable XLSX workbook.
pub trait XlsxWorkbook: Send + Sync {
    /// Add a new sheet to the workbook.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the sheet cannot be added (e.g. duplicate name).
    fn add_sheet(&mut self, name: &str) -> Result<(), DomainError>;

    /// Write a string value to a cell.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the cell coordinates are invalid.
    fn write_string(&mut self, row: u32, col: u16, value: &str) -> Result<(), DomainError>;

    /// Write a numeric value to a cell.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the cell coordinates are invalid.
    fn write_number(&mut self, row: u32, col: u16, value: f64) -> Result<(), DomainError>;

    /// Save the workbook to the given path.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if the file cannot be written.
    fn save(self: Box<Self>, path: &Path) -> Result<(), DomainError>;
}
