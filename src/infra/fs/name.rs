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
    fn test_get_valid_fs_name() {
        assert_eq!(get_valid_fs_name("Artist - Title"), "Artist - Title");
        assert_eq!(get_valid_fs_name("Artist: Title"), "Artist： Title");
        assert_eq!(get_valid_fs_name("Test/File"), "Test／File");
        assert_eq!(get_valid_fs_name("Test\\File"), "Test＼File");
        assert_eq!(get_valid_fs_name("Test*File"), "Test＊File");
        assert_eq!(get_valid_fs_name("Test?File"), "Test？File");
        assert_eq!(get_valid_fs_name("Test!File"), "Test！File");
        assert_eq!(get_valid_fs_name("Test\"File"), "Test＂File");
        assert_eq!(get_valid_fs_name("Test<File"), "Test＜File");
        assert_eq!(get_valid_fs_name("Test>File"), "Test＞File");
        assert_eq!(get_valid_fs_name("Test|File"), "Test｜File");
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
    fn test_get_valid_fs_name_empty() {
        assert_eq!(get_valid_fs_name(""), "");
    }

    #[test]
    fn test_get_valid_fs_name_cjk_unchanged() {
        assert_eq!(get_valid_fs_name("日本語"), "日本語");
        assert_eq!(get_valid_fs_name("中文測試"), "中文測試");
    }
}
