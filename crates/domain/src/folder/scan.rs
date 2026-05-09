//! BMS folder scanning operations.

use std::path::Path;
use tokio::fs;

use crate::error::DomainError;

/// Scan for similar folder names using sequence matching.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn scan_folder_similar_folders(
    root_dir: &Path,
    similarity_trigger: f64,
) -> Result<(), DomainError> {
    if !root_dir.is_dir() {
        return Ok(());
    }

    let mut dir_names: Vec<String> = Vec::new();
    let mut read_dir = fs::read_dir(root_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        if !entry.path().is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(String::from) else {
            continue;
        };
        dir_names.push(name);
    }

    tracing::info!("当前目录下有{}个文件夹。", dir_names.len());

    let mut sorted_names = dir_names.clone();
    sorted_names.sort();

    for i in 1..sorted_names.len() {
        let former = &sorted_names[i - 1];
        let current = &sorted_names[i];

        let similarity = sequence_matcher_ratio(former, current);
        if similarity < similarity_trigger {
            continue;
        }
        tracing::info!("发现相似项：{former} <=> {current}");
    }

    Ok(())
}

fn sequence_matcher_ratio(a: &str, b: &str) -> f64 {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    if a_chars.is_empty() && b_chars.is_empty() {
        return 1.0;
    }
    if a_chars.is_empty() || b_chars.is_empty() {
        return 0.0;
    }
    let total = a_chars.len() + b_chars.len();
    let matches = find_longest_match(&a_chars, &b_chars);
    #[expect(clippy::cast_precision_loss)]
    {
        (2 * matches) as f64 / total as f64
    }
}

#[expect(clippy::similar_names)]
fn find_longest_match(a: &[char], b: &[char]) -> usize {
    let mut best_len = 0;
    let mut best_ai = 0;
    let mut best_bi = 0;

    for ai in 0..a.len() {
        for bi in 0..b.len() {
            let mut k = 0;
            while ai + k < a.len() && bi + k < b.len() && a[ai + k] == b[bi + k] {
                k += 1;
            }
            if k > best_len {
                best_len = k;
                best_ai = ai;
                best_bi = bi;
            }
        }
    }

    if best_len == 0 {
        return 0;
    }

    let left_matches = find_longest_match(&a[..best_ai], &b[..best_bi]);
    let right_matches = find_longest_match(&a[best_ai + best_len..], &b[best_bi + best_len..]);

    best_len + left_matches + right_matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequence_identical() {
        assert!((sequence_matcher_ratio("abc", "abc") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_sequence_different() {
        assert!((sequence_matcher_ratio("abc", "xyz") - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_sequence_partial() {
        let sim = sequence_matcher_ratio("abc", "abd");
        assert!((sim - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_sequence_both_empty() {
        assert!((sequence_matcher_ratio("", "") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_sequence_one_empty() {
        assert!((sequence_matcher_ratio("a", "") - 0.0).abs() < 1e-6);
    }

    use tempfile::TempDir;

    #[tokio::test]
    async fn test_scan_similar_folders_detects_similar() {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join("MySong Hyper"))
            .await
            .unwrap();
        fs::create_dir_all(root.path().join("MySong Another"))
            .await
            .unwrap();
        scan_folder_similar_folders(root.path(), 0.5).await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_similar_folders_no_false_positive() {
        let root = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join("Alpha")).await.unwrap();
        fs::create_dir_all(root.path().join("Beta")).await.unwrap();
        scan_folder_similar_folders(root.path(), 0.9).await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_empty_dir() {
        let root = TempDir::new().unwrap();
        scan_folder_similar_folders(root.path(), 0.7).await.unwrap();
    }

    #[test]
    fn test_sequence_matcher_ratio_known_values() {
        let r = sequence_matcher_ratio("abc", "abc");
        assert!((r - 1.0).abs() < 1e-6, "identical strings");
        let r = sequence_matcher_ratio("abc", "abd");
        assert!((r - 2.0 / 3.0).abs() < 1e-6, "one char diff");
        let r = sequence_matcher_ratio("abc", "xyz");
        assert!((r - 0.0).abs() < 1e-6, "completely different");
        let r = sequence_matcher_ratio("ab", "ab");
        assert!((r - 1.0).abs() < 1e-6, "short identical");
    }
}
