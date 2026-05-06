use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::domain::error::DomainError;
use crate::domain::port::{FsPort, OutputPort};
use crate::infra::fs::pack_move::{
    MoveOptions, ReplaceOptions, is_dir_having_file, move_elements_across_dir,
};

/// Regular expression for Japanese Hiragana
const RE_JAPANESE_HIRAGANA: &str = r"[぀-ゟ]+";
/// Regular expression for Japanese Katakana
const RE_JAPANESE_KATAKANA: &str = r"[゠-ヿ]+";
/// Regular expression for Chinese characters
const RE_CHINESE_CHARACTER: &str = r"[一-龥]+";

static RE_HIRAGANA: LazyLock<Regex> = LazyLock::new(|| Regex::new(RE_JAPANESE_HIRAGANA).unwrap());
static RE_KATAKANA: LazyLock<Regex> = LazyLock::new(|| Regex::new(RE_JAPANESE_KATAKANA).unwrap());
static RE_CHINESE: LazyLock<Regex> = LazyLock::new(|| Regex::new(RE_CHINESE_CHARACTER).unwrap());

fn _check_range(name: &str, start: char, end: char) -> bool {
    name.chars()
        .next()
        .is_some_and(|c| c.to_ascii_uppercase() >= start && c.to_ascii_uppercase() <= end)
}

/// Find the first character rule group for a name
fn find_first_char_rule(name: &str) -> String {
    if name.is_empty() {
        return "未分类".to_string();
    }

    let first_char = name.chars().next().unwrap();

    if first_char.is_ascii_digit() {
        return "0-9".to_string();
    }

    if first_char.is_ascii_alphabetic() {
        let upper = first_char.to_ascii_uppercase();
        if ('A'..='D').contains(&upper) {
            return "ABCD".to_string();
        }
        if ('E'..='K').contains(&upper) {
            return "EFGHIJK".to_string();
        }
        if ('L'..='Q').contains(&upper) {
            return "LMNOPQ".to_string();
        }
        if ('R'..='T').contains(&upper) {
            return "RST".to_string();
        }
        if ('U'..='Z').contains(&upper) {
            return "UVWXYZ".to_string();
        }
    }

    let c_str = first_char.to_string();
    if RE_HIRAGANA.is_match(&c_str) {
        return "平假名".to_string();
    }
    if RE_KATAKANA.is_match(&c_str) {
        return "片假名".to_string();
    }
    if RE_CHINESE.is_match(&c_str) {
        return "汉字".to_string();
    }

    "+".to_string()
}

/// Service for splitting BMS packs.
pub struct PackSplitService {
    fs: Box<dyn FsPort>,
    output: Box<dyn OutputPort>,
}

impl PackSplitService {
    /// Create a new `PackSplitService`.
    #[must_use]
    pub fn new(fs: Box<dyn FsPort>, output: Box<dyn OutputPort>) -> Self {
        Self { fs, output }
    }

    /// Split folders in `root_dir` into subdirectories based on first character.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if directory operations fail.
    pub async fn split_folders_with_first_char(&self, root_dir: &Path) -> Result<(), DomainError> {
        if !self.fs.is_dir(root_dir).await {
            self.output
                .info(&format!("{} is not a dir! Aborting...", root_dir.display()));
            return Ok(());
        }

        let root_folder_name = root_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if root_folder_name.ends_with(']') {
            self.output
                .info(&format!("{} endswith ']'. Aborting...", root_dir.display()));
            return Ok(());
        }

        let Some(parent_dir) = root_dir.parent() else {
            return Ok(());
        };

        let entries = self.fs.read_dir(root_dir).await?;

        for entry in &entries {
            let element_name = &entry.name;
            let rule = find_first_char_rule(element_name);
            let target_dir = parent_dir.join(format!("{root_folder_name} [{rule}]"));

            if !self.fs.is_dir(&target_dir).await {
                self.fs.create_dir_all(&target_dir).await?;
            }

            let target_path = target_dir.join(element_name);
            self.output
                .info(&format!("Moving {:?} -> {:?}", entry.path, target_path));
            self.fs.rename(&entry.path, &target_path).await?;
        }

        if !is_dir_having_file(root_dir).await {
            let _ = self.fs.remove_dir(root_dir).await;
        }

        Ok(())
    }

    /// Undo split pack operation - move folders back from categorized subdirs.
    ///
    /// # Errors
    ///
    /// Returns [`DomainError`] if directory operations fail.
    pub async fn undo_split_pack(&self, root_dir: &Path) -> Result<(), DomainError> {
        let root_folder_name = root_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let Some(parent_dir) = root_dir.parent() else {
            return Ok(());
        };

        let mut pairs: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();

        if let Ok(entries) = self.fs.read_dir(parent_dir).await {
            for entry in &entries {
                if !entry.is_dir {
                    continue;
                }

                if entry.name.starts_with(&format!("{root_folder_name} ["))
                    && entry.name.ends_with(']')
                {
                    self.output.info(&format!(
                        " - {} <- {}",
                        root_dir.display(),
                        entry.path.display()
                    ));
                    pairs.push((entry.path.clone(), root_dir.to_path_buf()));
                }
            }
        }

        if pairs.is_empty() {
            return Ok(());
        }

        for (from, to) in &pairs {
            move_elements_across_dir(from, to, MoveOptions::default(), &ReplaceOptions::default())
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::service::test_util::MockOutput;
    use crate::infra::adapters::fs::TokioFsAdapter;
    use tempfile::TempDir;
    use tokio::fs;

    // --- Pure function tests ---

    #[test]
    fn test_find_first_char_rule_digit() {
        assert_eq!(find_first_char_rule("1stSong"), "0-9");
    }

    #[test]
    fn test_find_first_char_rule_abcd() {
        assert_eq!(find_first_char_rule("AlphaSong"), "ABCD");
        assert_eq!(find_first_char_rule("beta"), "ABCD");
    }

    #[test]
    fn test_find_first_char_rule_efghijk() {
        assert_eq!(find_first_char_rule("Eagle"), "EFGHIJK");
        assert_eq!(find_first_char_rule("kite"), "EFGHIJK");
    }

    #[test]
    fn test_find_first_char_rule_lmnopq() {
        assert_eq!(find_first_char_rule("Lion"), "LMNOPQ");
    }

    #[test]
    fn test_find_first_char_rule_rst() {
        assert_eq!(find_first_char_rule("Rock"), "RST");
    }

    #[test]
    fn test_find_first_char_rule_uvwxyz() {
        assert_eq!(find_first_char_rule("Zoo"), "UVWXYZ");
    }

    #[test]
    fn test_find_first_char_rule_empty() {
        assert_eq!(find_first_char_rule(""), "未分类");
    }

    #[test]
    fn test_find_first_char_rule_other() {
        assert_eq!(find_first_char_rule("+test"), "+");
    }

    // --- I/O tests ---

    #[tokio::test]
    async fn test_split_by_first_char() {
        let parent = TempDir::new().unwrap();
        let root = parent.path().join("split_char");
        fs::create_dir_all(&root).await.unwrap();
        let a_dir = root.join("AlphaSong");
        let b_dir = root.join("BetaSong");
        fs::create_dir_all(&a_dir).await.unwrap();
        fs::create_dir_all(&b_dir).await.unwrap();

        let service = PackSplitService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.split_folders_with_first_char(&root).await.unwrap();

        let abcd_dir = parent.path().join("split_char [ABCD]");
        assert!(abcd_dir.is_dir(), "ABCD group should exist");
        assert!(abcd_dir.join("AlphaSong").is_dir());
        assert!(abcd_dir.join("BetaSong").is_dir());
    }

    #[tokio::test]
    async fn test_undo_split_pack() {
        let parent = TempDir::new().unwrap();
        let root = parent.path().join("undo_split");
        fs::create_dir_all(&root).await.unwrap();
        let group_dir = parent.path().join("undo_split [ABCD]");
        fs::create_dir_all(&group_dir).await.unwrap();
        fs::write(group_dir.join("SongName.bms"), "#TITLE Song\n")
            .await
            .unwrap();

        let service = PackSplitService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        service.undo_split_pack(&root).await.unwrap();

        assert!(
            root.join("SongName.bms").is_file(),
            "file should be moved back to root"
        );
    }
}
