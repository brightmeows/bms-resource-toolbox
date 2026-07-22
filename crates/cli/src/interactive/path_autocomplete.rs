//! Directory autocomplete for inquire.
//!
//! Provides tab-completion that only shows directories (not files).

use std::path::Path;

/// An [`inquire::Autocomplete`] implementation that completes directory paths.
#[derive(Clone)]
pub struct DirAutocomplete;

impl inquire::Autocomplete for DirAutocomplete {
    fn get_suggestions(
        &mut self,
        input: &str,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let input_path = Path::new(input);

        let (scan_dir, prefix) = if input.ends_with('/') || input.is_empty() {
            let dir = if input.is_empty() {
                Path::new(".")
            } else {
                input_path
            };
            (dir.to_path_buf(), String::new())
        } else if let Some(parent) = input_path.parent() {
            let parent_str = parent.to_string_lossy();
            let parent_path = if parent_str.is_empty() {
                Path::new(".")
            } else {
                parent
            };
            let last_component = input_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            (parent_path.to_path_buf(), last_component)
        } else {
            return Ok(Vec::new());
        };

        let Ok(mut entries) = std::fs::read_dir(&scan_dir) else {
            return Ok(Vec::new());
        };

        let mut suggestions: Vec<String> = Vec::new();

        while let Some(Ok(entry)) = entries.next() {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_dir() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();

            if !name.starts_with(&prefix) {
                continue;
            }

            let full_path = if scan_dir.to_string_lossy() == "." && !prefix.is_empty() {
                name.clone()
            } else if scan_dir.to_string_lossy() == "." {
                format!("./{name}/")
            } else {
                let base = scan_dir.join(&name);
                format!("{}/", base.to_string_lossy())
            };

            suggestions.push(full_path);
        }

        suggestions.sort();
        Ok(suggestions)
    }

    fn get_completion(
        &mut self,
        input: &str,
        _highlighted_suggestion: Option<String>,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        if input.ends_with('/') || input.is_empty() {
            return Ok(Some(input.to_string()));
        }
        Ok(Some(input.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inquire::Autocomplete;
    use tempfile::TempDir;

    #[test]
    fn test_empty_input_shows_dot_entries() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("subdir")).unwrap();
        std::fs::write(tmp.path().join("file.txt"), "data").unwrap();

        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        let mut ac = DirAutocomplete;
        let suggestions = ac.get_suggestions("").unwrap();
        std::env::set_current_dir(orig).unwrap();

        assert!(
            suggestions.iter().any(|s| s.contains("subdir")),
            "should contain subdir, got: {suggestions:?}"
        );
        assert!(
            !suggestions.iter().any(|s| s.contains("file.txt")),
            "should not contain files"
        );
    }

    #[test]
    fn test_partial_prefix_filtering() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("alpha")).unwrap();
        std::fs::create_dir_all(tmp.path().join("beta")).unwrap();

        let mut ac = DirAutocomplete;
        let path_str = tmp.path().join("al").to_string_lossy().to_string();
        let suggestions = ac.get_suggestions(&path_str).unwrap();

        assert_eq!(suggestions.len(), 1, "only alpha should match");
    }
}
