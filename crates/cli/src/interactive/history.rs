//! Path history persistence using XDG data directory.
//!
//! Stores recently used paths in `$XDG_DATA_HOME/bms-res-tb/history.log`
//! as JSON Lines format (one JSON-string per line).
//!
//! # Sync fs usage
//!
//! This module runs synchronously inside the interactive prompt chain.
//! All `std::fs` calls are intentional and preceded by `#[expect]`.

use std::path::PathBuf;

use serde_json;

const HISTORY_FILE: &str = "bms-res-tb/history.log";
const MAX_HISTORY: usize = 50;
const DISPLAY_COUNT: usize = 5;

/// Test-only override for data directory.
#[cfg(test)]
static TEST_DATA_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

/// Get the XDG data directory path for history.
fn data_dir() -> PathBuf {
    // Use test override if set
    #[cfg(test)]
    {
        if let Some(path) = TEST_DATA_DIR.lock().unwrap().clone() {
            return path;
        }
    }

    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        let p = PathBuf::from(xdg);
        if !p.as_os_str().is_empty() {
            return p;
        }
    }
    // Fallback: ~/.local/share
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".local/share")
}

/// Get the full path to the history file.
fn history_path() -> PathBuf {
    data_dir().join(HISTORY_FILE)
}

/// Read all history entries from the history file.
///
/// Returns entries in order from most recent to oldest.
fn read_history() -> Vec<String> {
    let path = history_path();
    if !path.is_file() {
        return Vec::new();
    }

    #[expect(clippy::disallowed_methods, reason = "sync interactive prompt")]
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Failed to read history file: {e}");
            return Vec::new();
        }
    };

    let mut entries: Vec<String> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(path) = serde_json::from_str::<String>(trimmed) {
            if !path.is_empty() {
                entries.push(path);
            }
        } else {
            // Fallback: treat raw line as path (for compatibility)
            let raw = trimmed.to_string();
            if !raw.is_empty() {
                entries.push(raw);
            }
        }
    }

    entries
}

/// Write history entries to the history file.
fn write_history(entries: &[String]) {
    let path = history_path();

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        #[expect(clippy::disallowed_methods, reason = "sync interactive prompt")]
        let _ = std::fs::create_dir_all(parent);
    }

    let content: String = entries
        .iter()
        .filter_map(|e| {
            if e.is_empty() {
                return None;
            }
            Some(
                serde_json::to_string(e)
                    .unwrap_or_else(|_| format!("\"{e}\""))
                    + "\n",
            )
        })
        .collect();

    #[expect(clippy::disallowed_methods, reason = "sync interactive prompt")]
    if let Err(e) = std::fs::write(&path, &content) {
        tracing::warn!("Failed to write history file: {e}");
    }
}

/// Add a path to history (dedup, move to top, trim to `MAX_HISTORY`).
pub fn add_to_history(path: &str) {
    if path.is_empty() {
        return;
    }

    let mut entries = read_history();

    // Remove existing entry if present
    entries.retain(|e| e != path);

    // Insert at top
    entries.insert(0, path.to_string());

    // Trim to max
    entries.truncate(MAX_HISTORY);

    write_history(&entries);
}

/// Display recent history entries and let user pick one or type a new path.
///
/// Returns the selected or entered path, or `None` if cancelled.
pub fn prompt_with_history(prompt: &str) -> Option<String> {
    let entries = read_history();

    if entries.is_empty() {
        let input = inquire::Text::new(prompt).prompt().ok()?;
        let trimmed = input.trim().to_string();
        if !trimmed.is_empty() {
            add_to_history(&trimmed);
        }
        return Some(trimmed);
    }

    // Build options: recent history entries + custom input + show all
    let display_count = DISPLAY_COUNT.min(entries.len());
    let recent = entries.get(..display_count).unwrap_or(&entries);

    let mut options: Vec<String> = Vec::new();
    for entry in recent {
        options.push(format!("🕐 {entry}"));
    }
    options.push("📝 输入新路径...".to_string());
    if entries.len() > display_count {
        options.push("📋 查看全部历史...".to_string());
    }
    options.push("🚪 取消".to_string());

    let Ok(selection) = inquire::Select::new(prompt, options).with_vim_mode(true).prompt() else {
        return None;
    };

    if let Some(path) = selection.strip_prefix("🕐 ") {
        let path = path.to_string();
        add_to_history(&path);
        return Some(path);
    }

    if selection == "📝 输入新路径..." {
        let input = inquire::Text::new("输入路径:")
            .with_autocomplete(super::path_autocomplete::DirAutocomplete)
            .prompt()
            .ok()?;
        let trimmed = input.trim().to_string();
        if !trimmed.is_empty() {
            add_to_history(&trimmed);
        }
        return Some(trimmed);
    }

    if selection == "📋 查看全部历史..." {
        return show_all_history();
    }

    None
}

/// Show all history entries in a scrollable list.
fn show_all_history() -> Option<String> {
    let entries = read_history();
    if entries.is_empty() {
        tracing::info!("暂无历史路径记录");
        return prompt_with_history("输入路径:");
    }

    let mut options: Vec<String> = entries.iter().map(|e| format!("🕐 {e}")).collect();
    options.push("🔙 返回".to_string());

    let Ok(selection) = inquire::Select::new("选择历史路径 (所有):", options)
        .with_vim_mode(true)
        .prompt()
    else {
        return None;
    };

    if let Some(path) = selection.strip_prefix("🕐 ") {
        let path = path.to_string();
        add_to_history(&path);
        return Some(path);
    }

    prompt_with_history("输入路径:")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_add_and_read_history() {
        let _lock = TEST_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        *super::TEST_DATA_DIR.lock().unwrap() = Some(tmp.path().to_path_buf());

        assert!(read_history().is_empty());

        add_to_history("/path/to/dir1");
        add_to_history("/path/to/dir2");

        let entries = read_history();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], "/path/to/dir2");
        assert_eq!(entries[1], "/path/to/dir1");

        add_to_history("/path/to/dir1");
        let entries = read_history();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], "/path/to/dir1");

        *super::TEST_DATA_DIR.lock().unwrap() = None;
    }

    #[test]
    fn test_history_max_entries() {
        let _lock = TEST_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        *super::TEST_DATA_DIR.lock().unwrap() = Some(tmp.path().to_path_buf());

        for i in 0..MAX_HISTORY + 10 {
            add_to_history(&format!("/path/{i}"));
        }
        let entries = read_history();
        assert_eq!(entries.len(), MAX_HISTORY);

        *super::TEST_DATA_DIR.lock().unwrap() = None;
    }

    #[test]
    fn test_empty_path_not_stored() {
        let _lock = TEST_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        *super::TEST_DATA_DIR.lock().unwrap() = Some(tmp.path().to_path_buf());

        add_to_history("");
        assert!(read_history().is_empty());

        *super::TEST_DATA_DIR.lock().unwrap() = None;
    }
}
