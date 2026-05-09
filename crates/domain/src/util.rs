//! Pure utility functions.

use std::path::Path;

/// Get the file extension of a path (without the dot).
///
/// Returns an empty string if the path has no recognizable extension.
#[must_use]
pub fn get_ext(path: &Path) -> &str {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.rsplit('.').next())
        .unwrap_or("")
}

/// Replace characters invalid in filenames with fullwidth alternatives.
///
/// Returns a sanitized string safe for use as a file or directory name.
#[must_use]
pub fn get_valid_fs_name(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            ':' => '：',
            '\\' => '＼',
            '/' => '／',
            '*' => '＊',
            '?' => '？',
            '!' => '！',
            '"' => '＂',
            '<' => '＜',
            '>' => '＞',
            '|' => '｜',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_ext_normal() {
        assert_eq!(get_ext(Path::new("file.txt")), "txt");
    }

    #[test]
    fn test_get_ext_no_dot_returns_full_name() {
        assert_eq!(get_ext(Path::new("Makefile")), "Makefile");
    }

    #[test]
    fn test_get_ext_multi_dot() {
        assert_eq!(get_ext(Path::new("archive.tar.gz")), "gz");
    }

    #[test]
    fn test_get_valid_fs_name_unchanged() {
        assert_eq!(get_valid_fs_name("Artist - Title"), "Artist - Title");
    }

    #[test]
    fn test_get_valid_fs_name_replace_colon() {
        assert_eq!(get_valid_fs_name("Artist: Title"), "Artist： Title");
    }

    #[test]
    fn test_get_valid_fs_name_all_replacements() {
        assert_eq!(get_valid_fs_name("A:B"), "A：B");
        assert_eq!(get_valid_fs_name("A\\B"), "A＼B");
        assert_eq!(get_valid_fs_name("A/B"), "A／B");
        assert_eq!(get_valid_fs_name("A*B"), "A＊B");
        assert_eq!(get_valid_fs_name("A?B"), "A？B");
        assert_eq!(get_valid_fs_name("A!B"), "A！B");
        assert_eq!(get_valid_fs_name("A\"B"), "A＂B");
        assert_eq!(get_valid_fs_name("A<B"), "A＜B");
        assert_eq!(get_valid_fs_name("A>B"), "A＞B");
        assert_eq!(get_valid_fs_name("A|B"), "A｜B");
    }

    #[test]
    fn test_get_valid_fs_name_cjk_unchanged() {
        assert_eq!(get_valid_fs_name("日本語"), "日本語");
        assert_eq!(get_valid_fs_name("中文測試"), "中文測試");
    }
}
