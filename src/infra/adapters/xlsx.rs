use std::path::Path;

use rust_xlsxwriter::{Workbook, Worksheet};

use crate::domain::error::DomainError;
use crate::domain::port::{XlsxPort, XlsxWorkbook};

/// Creates xlsx workbooks using `rust_xlsxwriter`.
pub struct RustXlsxWriterAdapter;

impl XlsxPort for RustXlsxWriterAdapter {
    fn create_workbook(&self) -> Box<dyn XlsxWorkbook> {
        Box::new(RustXlsxWorkbook::new())
    }
}

struct RustXlsxWorkbook {
    workbook: Workbook,
    worksheets: Vec<Worksheet>,
}

impl RustXlsxWorkbook {
    fn new() -> Self {
        Self {
            workbook: Workbook::new(),
            worksheets: Vec::new(),
        }
    }
}

impl XlsxWorkbook for RustXlsxWorkbook {
    fn add_sheet(&mut self, name: &str) -> Result<(), DomainError> {
        let mut sheet = Worksheet::new();
        sheet
            .set_name(name)
            .map_err(|e| DomainError::Archive(anyhow::anyhow!("{e}")))?;
        self.worksheets.push(sheet);
        Ok(())
    }

    fn write_string(&mut self, row: u32, col: u16, value: &str) -> Result<(), DomainError> {
        let sheet = self
            .worksheets
            .last_mut()
            .ok_or_else(|| DomainError::Archive(anyhow::anyhow!("no active worksheet")))?;
        sheet
            .write_string(row, col, value)
            .map_err(|e| DomainError::Archive(anyhow::anyhow!("{e}")))?;
        Ok(())
    }

    fn write_number(&mut self, row: u32, col: u16, value: f64) -> Result<(), DomainError> {
        let sheet = self
            .worksheets
            .last_mut()
            .ok_or_else(|| DomainError::Archive(anyhow::anyhow!("no active worksheet")))?;
        sheet
            .write_number(row, col, value)
            .map_err(|e| DomainError::Archive(anyhow::anyhow!("{e}")))?;
        Ok(())
    }

    fn save(self: Box<Self>, path: &Path) -> Result<(), DomainError> {
        let mut wb = self.workbook;
        for ws in self.worksheets {
            wb.push_worksheet(ws);
        }
        wb.save(path)
            .map_err(|e| DomainError::Archive(anyhow::anyhow!("{e}")))?;
        Ok(())
    }
}
