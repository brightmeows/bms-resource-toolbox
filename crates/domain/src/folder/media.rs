//! BMS media file cleanup operations.
//!
//! This module provides functions for removing unneeded media files
//! from BMS work directories based on format priority rules.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Semaphore;

use crate::error::DomainError;

/// Media file removal rule
pub type RemoveMediaRule = Vec<(Vec<&'static str>, Vec<&'static str>)>;

/// ORAJA removal rule - remove redundant video files and prefer specific formats
#[must_use]
pub fn get_remove_media_rule_oraja() -> RemoveMediaRule {
    vec![
        (vec!["mp4"], vec!["avi", "wmv", "mpg", "mpeg"]),
        (vec!["avi"], vec!["wmv", "mpg", "mpeg"]),
        (vec!["flac", "wav"], vec!["ogg"]),
        (vec!["flac"], vec!["wav"]),
        (vec!["mpg"], vec!["wmv"]),
    ]
}

/// WAV→FLAC removal rule: if WAV exists, remove corresponding FLAC.
#[must_use]
pub fn get_remove_media_rule_wav_flac() -> RemoveMediaRule {
    vec![(vec!["wav"], vec!["flac"])]
}

/// MPG→WMV removal rule: if MPG exists, remove corresponding WMV.
#[must_use]
pub fn get_remove_media_rule_mpg_wmv() -> RemoveMediaRule {
    vec![(vec!["mpg"], vec!["wmv"])]
}

async fn workdir_remove_unneed_media_files(
    work_dir: &Path,
    rule: &RemoveMediaRule,
) -> Result<(), DomainError> {
    let mut remove_pairs: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut removed_files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    let mut read_dir = fs::read_dir(work_dir).await?;
    let mut entries = Vec::new();
    while let Some(entry) = read_dir.next_entry().await? {
        entries.push(entry);
    }

    for entry in &entries {
        let check_file_path = entry.path();
        if !check_file_path.is_file() {
            continue;
        }

        let file_ext = check_file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        for (upper_exts, lower_exts) in rule {
            if !upper_exts.contains(&file_ext) {
                continue;
            }

            if fs::metadata(&check_file_path)
                .await
                .is_ok_and(|m| m.len() == 0)
            {
                tracing::info!(" - !x!: File {check_file_path:?} is Empty! Skipping...");
                continue;
            }

            for lower_ext in lower_exts {
                let replacing_file_path = check_file_path.with_extension(*lower_ext);
                if !replacing_file_path.is_file() {
                    continue;
                }
                if removed_files.contains(&replacing_file_path) {
                    continue;
                }
                remove_pairs.push((check_file_path.clone(), replacing_file_path.clone()));
                removed_files.insert(replacing_file_path);
            }
        }
    }

    for (check_file_path, replacing_file_path) in &remove_pairs {
        tracing::info!(
            "- Remove file {:?}, because {:?} exists.",
            replacing_file_path.file_name(),
            check_file_path.file_name()
        );
        let _ = fs::remove_file(replacing_file_path).await;
    }

    super::cleanup::remove_zero_sized_media_files(work_dir, false).await?;

    let mut ext_count: HashMap<String, Vec<String>> = HashMap::new();
    let mut count_read_dir = fs::read_dir(work_dir).await?;
    while let Some(entry) = count_read_dir.next_entry().await? {
        let count_file_path = entry.path();
        if !count_file_path.is_file() {
            continue;
        }
        let file_ext = count_file_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let file_name = count_file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        ext_count
            .entry(file_ext.to_string())
            .or_default()
            .push(file_name);
    }

    if let Some(mp4_files) = ext_count.get("mp4")
        && mp4_files.len() > 1
    {
        tracing::info!(" - Tips: {work_dir:?} has more than 1 mp4 files!");
    }

    Ok(())
}

/// Remove unneeded media files from all works in `root_dir`
///
/// # Errors
///
/// Returns an error if directory operations fail.
///
/// # Panics
///
/// Panics if the internal concurrency semaphore is closed, which should not
/// happen under normal operation.
pub async fn remove_unneed_media_files(
    root_dir: &Path,
    rule: RemoveMediaRule,
) -> Result<(), DomainError> {
    tracing::info!("Selected: {rule:?}");

    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut read_dir = fs::read_dir(root_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        if entry.path().is_dir() {
            dirs.push(entry.path());
        }
    }

    if dirs.is_empty() {
        return Ok(());
    }

    let cpu_count = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let sem = Arc::new(Semaphore::new(cpu_count));
    let mut handles = Vec::with_capacity(dirs.len());

    for dir_path in dirs {
        let sem_clone = sem.clone();
        let rule_clone = rule.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.expect("semaphore not closed");
            workdir_remove_unneed_media_files(&dir_path, &rule_clone)
                .await
                .map_err(|e| (dir_path, e))
        }));
    }

    let mut errors: Vec<(PathBuf, DomainError)> = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err((dir, e))) => {
                tracing::info!(" - Dir: {dir:?} Error occured!");
                errors.push((dir, e));
            }
            Err(e) => {
                return Err(std::io::Error::other(format!("Task join error: {e}")).into());
            }
        }
    }

    if let Some((_, e)) = errors.into_iter().next() {
        return Err(e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn workdir_path(root: &Path) -> PathBuf {
        let wd = root.join("work");
        fs::create_dir_all(&wd).await.unwrap();
        wd
    }

    #[tokio::test]
    async fn test_remove_media_mp4_removes_avi() {
        let dir = TempDir::new().unwrap();
        let wd = workdir_path(dir.path()).await;
        fs::write(wd.join("v.mp4"), "mp4").await.unwrap();
        fs::write(wd.join("v.avi"), "avi").await.unwrap();
        remove_unneed_media_files(dir.path(), get_remove_media_rule_oraja())
            .await
            .unwrap();
        assert!(!wd.join("v.avi").exists());
    }

    #[tokio::test]
    async fn test_remove_media_flac_removes_wav() {
        let dir = TempDir::new().unwrap();
        let wd = workdir_path(dir.path()).await;
        fs::write(wd.join("a.flac"), "flac").await.unwrap();
        fs::write(wd.join("a.wav"), "wav").await.unwrap();
        remove_unneed_media_files(dir.path(), get_remove_media_rule_oraja())
            .await
            .unwrap();
        assert!(!wd.join("a.wav").exists());
    }

    #[tokio::test]
    async fn test_remove_media_flac_wav_removes_ogg() {
        let dir = TempDir::new().unwrap();
        let wd = workdir_path(dir.path()).await;
        fs::write(wd.join("a.flac"), "flac").await.unwrap();
        fs::write(wd.join("a.ogg"), "ogg").await.unwrap();
        remove_unneed_media_files(dir.path(), get_remove_media_rule_oraja())
            .await
            .unwrap();
        assert!(!wd.join("a.ogg").exists());
    }

    #[tokio::test]
    async fn test_remove_media_mp4_removes_wmv() {
        let dir = TempDir::new().unwrap();
        let wd = workdir_path(dir.path()).await;
        fs::write(wd.join("v.mp4"), "mp4").await.unwrap();
        fs::write(wd.join("v.wmv"), "wmv").await.unwrap();
        remove_unneed_media_files(dir.path(), get_remove_media_rule_oraja())
            .await
            .unwrap();
        assert!(!wd.join("v.wmv").exists());
    }

    #[tokio::test]
    async fn test_remove_media_no_duplicate_no_removal() {
        let dir = TempDir::new().unwrap();
        let wd = workdir_path(dir.path()).await;
        fs::write(wd.join("a.flac"), "flac").await.unwrap();
        remove_unneed_media_files(dir.path(), get_remove_media_rule_oraja())
            .await
            .unwrap();
        assert!(wd.join("a.flac").is_file());
    }

    #[test]
    fn test_get_remove_media_rule_wav_flac() {
        let rule = get_remove_media_rule_wav_flac();
        assert_eq!(rule.len(), 1);
        assert_eq!(rule[0].0, vec!["wav"]);
        assert_eq!(rule[0].1, vec!["flac"]);
    }

    #[test]
    fn test_get_remove_media_rule_mpg_wmv() {
        let rule = get_remove_media_rule_mpg_wmv();
        assert_eq!(rule.len(), 1);
        assert_eq!(rule[0].0, vec!["mpg"]);
        assert_eq!(rule[0].1, vec!["wmv"]);
    }
}
