use std::path::Path;

use crate::domain::bms::info::get_dir_bms_info;
use crate::domain::error::DomainError;
use crate::domain::port::{FsPort, OutputPort, XlsxPort};

/// Service for BMS event operations: numbered folder management and xlsx table generation.
pub struct EventService {
    fs: Box<dyn FsPort>,
    output: Box<dyn OutputPort>,
    xlsx: Box<dyn XlsxPort>,
}

impl EventService {
    /// Create a new `EventService` with the given port implementations.
    #[must_use]
    pub fn new(fs: Box<dyn FsPort>, output: Box<dyn OutputPort>, xlsx: Box<dyn XlsxPort>) -> Self {
        Self { fs, output, xlsx }
    }

    /// Check which numbered folders (`1..=max_count`) exist in `bms_dir`.
    /// Reports missing folders via the output port.
    pub async fn check_num_folder(&self, bms_dir: &Path, max_count: i32) {
        for i in 1..=max_count {
            let folder_path = bms_dir.join(format!("{i}"));
            if !self.fs.is_dir(&folder_path).await {
                self.output
                    .info(&format!("{} is not exist!", folder_path.display()));
            }
        }
    }

    /// Create numbered folders (`1..=folder_count`) in `root_dir`.
    /// Skips folders that conflict with existing entries (same prefix).
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation fails.
    pub async fn create_num_folders(
        &self,
        root_dir: &Path,
        folder_count: i32,
    ) -> Result<(), DomainError> {
        self.output.info(&format!(
            "Creating {folder_count} numbered folders in {root_dir:?}"
        ));

        let mut existing_elements: Vec<String> = Vec::new();
        if let Ok(entries) = self.fs.read_dir(root_dir).await {
            for entry in entries {
                if !entry.is_dir {
                    continue;
                }
                existing_elements.push(entry.name);
            }
        }

        for i in 1..=folder_count {
            let folder_name = format!("{i}");
            let folder_path = root_dir.join(&folder_name);

            let id_exists = existing_elements.iter().any(|element_name| {
                element_name == &folder_name
                    || element_name.starts_with(&format!("{folder_name}."))
                    || element_name.starts_with(&format!("{folder_name} "))
            });

            if id_exists {
                self.output.info(&format!(
                    "  Folder {i} conflicts with existing entry, skipping"
                ));
                continue;
            }

            self.fs.create_dir_all(&folder_path).await?;
            self.output.info(&format!("  Created folder {i}"));
        }

        Ok(())
    }

    /// Generate a work info table (xlsx) for BMS works in `root_dir`.
    /// Iterates through subdirectories, extracts BMS metadata, and writes to `bms_list.xlsx`.
    ///
    /// # Errors
    ///
    /// Returns an error if directory operations or xlsx generation fail.
    pub async fn generate_work_info_table(&self, root_dir: &Path) -> Result<(), DomainError> {
        self.output
            .info(&format!("Generating work info table for: {root_dir:?}"));

        let mut workbook = self.xlsx.create_workbook();
        workbook.add_sheet("BMS List")?;

        let entries = self.fs.read_dir(root_dir).await?;
        for entry in entries {
            if !entry.is_dir {
                continue;
            }

            let work_name = entry.name.as_str();
            let Some(info) = get_dir_bms_info(&*self.fs, &entry.path).await else {
                continue;
            };

            let id_str = work_name.split('.').next().unwrap_or("");
            if id_str.is_empty() || !id_str.chars().all(|c| c.is_ascii_digit()) {
                self.output.info(&format!(
                    "Warning: Skipping dir {work_name} - invalid id format: {id_str}"
                ));
                continue;
            }
            let id_num: u32 = id_str.parse().unwrap_or(0);
            if id_num == 0 {
                continue;
            }
            let row = id_num - 1;

            workbook.write_number(row, 0, f64::from(id_num))?;
            workbook.write_string(row, 1, &info.title)?;
            workbook.write_string(row, 2, &info.artist)?;
            workbook.write_string(row, 3, &info.genre)?;
        }

        let table_path = root_dir.join("bms_list.xlsx");
        self.output
            .info(&format!("Saving table to {}", table_path.display()));
        workbook.save(&table_path)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::service::test_util::MockOutput;
    use crate::infra::adapters::fs::TokioFsAdapter;
    use crate::infra::adapters::xlsx::RustXlsxWriterAdapter;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_check_num_folder_exist() {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join("1")).await.unwrap();
        fs::create_dir_all(root.path().join("2")).await.unwrap();
        let service = EventService::new(
            Box::new(TokioFsAdapter),
            Box::new(MockOutput::new()),
            Box::new(RustXlsxWriterAdapter),
        );
        service.check_num_folder(root.path(), 3).await;
    }

    #[tokio::test]
    async fn test_create_num_folders() {
        let root = TempDir::new().unwrap();
        let service = EventService::new(
            Box::new(TokioFsAdapter),
            Box::new(MockOutput::new()),
            Box::new(RustXlsxWriterAdapter),
        );
        service.create_num_folders(root.path(), 5).await.unwrap();
        for i in 1..=5 {
            assert!(
                root.path().join(i.to_string()).is_dir(),
                "folder {i} should exist"
            );
        }
    }

    #[tokio::test]
    async fn test_create_num_folders_skip_conflict() {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join("1. Title"))
            .await
            .unwrap();
        let service = EventService::new(
            Box::new(TokioFsAdapter),
            Box::new(MockOutput::new()),
            Box::new(RustXlsxWriterAdapter),
        );
        service.create_num_folders(root.path(), 3).await.unwrap();
        assert!(root.path().join("2").is_dir());
        assert!(root.path().join("3").is_dir());
    }

    #[tokio::test]
    async fn test_generate_work_info_table() {
        let root = TempDir::new().unwrap();
        for i in 1..=3 {
            let dir = root.path().join(i.to_string());
            fs::create_dir_all(&dir).await.unwrap();
            fs::write(
                dir.join("test.bms"),
                format!("#TITLE Song{i}\n#ARTIST Artist{i}\n#GENRE Genre{i}\n"),
            )
            .await
            .unwrap();
        }
        let service = EventService::new(
            Box::new(TokioFsAdapter),
            Box::new(MockOutput::new()),
            Box::new(RustXlsxWriterAdapter),
        );
        service.generate_work_info_table(root.path()).await.unwrap();
        let xlsx = root.path().join("bms_list.xlsx");
        assert!(xlsx.is_file(), "xlsx should be generated");
    }
}
