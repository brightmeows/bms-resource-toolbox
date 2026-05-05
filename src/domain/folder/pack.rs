//! BMS big pack operations.
//!
//! This module provides functions for managing large BMS packs
//! including folder splitting, merging, and work movement.

use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

use crate::domain::error::DomainError;
use crate::infra::fs::pack_move::{
    MoveOptions, REPLACE_OPTION_UPDATE_PACK, ReplaceOptions, is_dir_having_file,
    move_elements_across_dir,
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

/// Split folders in `root_dir` into subdirectories based on first character
///
/// # Errors
///
/// Returns [`std::io::Error`] if directory operations fail.
pub async fn split_folders_with_first_char(root_dir: &Path) -> Result<(), DomainError> {
    if !root_dir.is_dir() {
        println!("{} is not a dir! Aborting...", root_dir.display());
        return Ok(());
    }

    let root_folder_name = root_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

    if root_folder_name.ends_with(']') {
        println!("{} endswith ']'. Aborting...", root_dir.display());
        return Ok(());
    }

    let Some(parent_dir) = root_dir.parent() else {
        return Ok(());
    };

    let mut read_dir = tokio::fs::read_dir(root_dir).await?;
    let mut entries = Vec::new();
    while let Some(entry) = read_dir.next_entry().await? {
        entries.push(entry);
    }

    for entry in entries {
        let element_path = entry.path();

        let element_name = element_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let rule = find_first_char_rule(element_name);
        let target_dir = parent_dir.join(format!("{root_folder_name} [{rule}]"));

        if !target_dir.is_dir() {
            tokio::fs::create_dir_all(&target_dir).await?;
        }

        let target_path = target_dir.join(element_name);
        println!("Moving {element_path:?} -> {target_path:?}");
        tokio::fs::rename(&element_path, &target_path).await?;
    }

    if !is_dir_having_file(root_dir).await {
        let _ = tokio::fs::remove_dir(root_dir).await;
    }

    Ok(())
}

/// Undo split pack operation - move folders back from categorized subdirs.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn undo_split_pack(root_dir: &Path) -> Result<(), DomainError> {
    let root_folder_name = root_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let Some(parent_dir) = root_dir.parent() else {
        return Ok(());
    };

    let mut pairs: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();

    if let Ok(mut read_dir) = tokio::fs::read_dir(parent_dir).await {
        while let Some(entry) = read_dir.next_entry().await? {
            let folder_path = entry.path();
            if !folder_path.is_dir() {
                continue;
            }
            let folder_name = folder_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if folder_name.starts_with(&format!("{root_folder_name} ["))
                && folder_name.ends_with(']')
            {
                println!(" - {} <- {}", root_dir.display(), folder_path.display());
                pairs.push((folder_path, root_dir.to_path_buf()));
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

/// Move works from one pack directory to another
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn move_works_in_pack(
    root_dir_from: &Path,
    root_dir_to: &Path,
) -> Result<(), DomainError> {
    if root_dir_from == root_dir_to {
        return Ok(());
    }

    let mut move_count = 0;

    if let Ok(mut read_dir) = tokio::fs::read_dir(root_dir_from).await {
        while let Some(entry) = read_dir.next_entry().await? {
            let bms_dir = entry.path();
            if !bms_dir.is_dir() {
                continue;
            }

            let bms_dir_name = bms_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

            println!("Moving: {bms_dir_name}");

            let dst_bms_dir = root_dir_to.join(bms_dir_name);
            move_elements_across_dir(
                &bms_dir,
                &dst_bms_dir,
                MoveOptions::default(),
                &REPLACE_OPTION_UPDATE_PACK,
            )
            .await?;
            move_count += 1;
        }
    }

    if move_count > 0 {
        println!("Move {move_count} songs.");
        return Ok(());
    }

    move_elements_across_dir(
        root_dir_from,
        root_dir_to,
        MoveOptions::default(),
        &REPLACE_OPTION_UPDATE_PACK,
    )
    .await?;

    Ok(())
}

/// Move works out one level (un-nest subdirectories)
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn move_out_works(target_root_dir: &Path) -> Result<(), DomainError> {
    let mut read_dir = tokio::fs::read_dir(target_root_dir).await?;
    let mut entries = Vec::new();
    while let Some(entry) = read_dir.next_entry().await? {
        entries.push(entry);
    }

    for entry in entries {
        let root_dir_path = entry.path();
        if !root_dir_path.is_dir() {
            continue;
        }

        let mut work_read_dir = tokio::fs::read_dir(&root_dir_path).await?;
        while let Some(work_entry) = work_read_dir.next_entry().await? {
            let work_dir_path = work_entry.path();

            let work_dir_name = work_dir_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            let target_work_dir_path = target_root_dir.join(work_dir_name);

            move_elements_across_dir(
                &work_dir_path,
                &target_work_dir_path,
                MoveOptions::default(),
                &REPLACE_OPTION_UPDATE_PACK,
            )
            .await?;
        }

        if !is_dir_having_file(&root_dir_path).await {
            let _ = tokio::fs::remove_dir(&root_dir_path).await;
        }
    }

    Ok(())
}

/// Move works with same name from one dir to another.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn move_works_with_same_name(
    root_dir_from: &Path,
    root_dir_to: &Path,
) -> Result<(), DomainError> {
    if !root_dir_from.is_dir() {
        return Err(DomainError::Archive(anyhow::anyhow!(
            "源路径不存在或不是目录: {}",
            root_dir_from.display()
        )));
    }
    if !root_dir_to.is_dir() {
        return Err(DomainError::Archive(anyhow::anyhow!(
            "目标路径不存在或不是目录: {}",
            root_dir_to.display()
        )));
    }

    let mut from_subdirs: Vec<(String, std::path::PathBuf)> = Vec::new();
    if let Ok(mut read_dir) = tokio::fs::read_dir(root_dir_from).await {
        while let Some(entry) = read_dir.next_entry().await? {
            if !entry.path().is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(String::from) else {
                continue;
            };
            from_subdirs.push((name, entry.path()));
        }
    }

    let mut to_subdirs: Vec<String> = Vec::new();
    if let Ok(mut read_dir) = tokio::fs::read_dir(root_dir_to).await {
        while let Some(entry) = read_dir.next_entry().await? {
            if !entry.path().is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(String::from) else {
                continue;
            };
            to_subdirs.push(name);
        }
    }

    let mut pairs: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();

    for (from_name, from_path) in &from_subdirs {
        for to_name in &to_subdirs {
            if to_name.starts_with(from_name) {
                let to_path = root_dir_to.join(to_name);
                println!(" -> {from_name} => {to_name}");
                pairs.push((from_path.clone(), to_path));
                break;
            }
        }
    }

    if pairs.is_empty() {
        return Ok(());
    }

    for (from_path, to_path) in &pairs {
        println!("合并: {} -> {}", from_path.display(), to_path.display());
        move_elements_across_dir(
            from_path,
            to_path,
            MoveOptions::default(),
            &REPLACE_OPTION_UPDATE_PACK,
        )
        .await?;
    }

    Ok(())
}

/// Move works with same name to sibling directories.
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn move_works_with_same_name_to_siblings(
    root_dir_from: &Path,
) -> Result<(), DomainError> {
    if !root_dir_from.is_dir() {
        return Err(DomainError::Archive(anyhow::anyhow!(
            "源路径不存在或不是目录: {}",
            root_dir_from.display()
        )));
    }

    let Some(parent_dir) = root_dir_from.parent() else {
        return Ok(());
    };

    let root_base_name = root_dir_from
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let mut from_subdirs: Vec<(String, std::path::PathBuf)> = Vec::new();
    if let Ok(mut read_dir) = tokio::fs::read_dir(root_dir_from).await {
        while let Some(entry) = read_dir.next_entry().await? {
            if !entry.path().is_dir() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(String::from) else {
                continue;
            };
            from_subdirs.push((name, entry.path()));
        }
    }

    let mut pairs: Vec<(std::path::PathBuf, std::path::PathBuf)> = Vec::new();

    if let Ok(mut siblings) = tokio::fs::read_dir(parent_dir).await {
        while let Some(sibling) = siblings.next_entry().await? {
            let sibling_path = sibling.path();
            if !sibling_path.is_dir() {
                continue;
            }

            let sibling_name = sibling_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if sibling_name == root_base_name {
                continue;
            }

            let mut to_subdirs: Vec<String> = Vec::new();
            if let Ok(mut read_dir) = tokio::fs::read_dir(&sibling_path).await {
                while let Some(entry) = read_dir.next_entry().await? {
                    if !entry.path().is_dir() {
                        continue;
                    }
                    let Some(name) = entry.file_name().to_str().map(String::from) else {
                        continue;
                    };
                    to_subdirs.push(name);
                }
            }

            for (from_name, from_path) in &from_subdirs {
                for to_name in &to_subdirs {
                    if to_name.starts_with(from_name) {
                        let target_path = sibling_path.join(to_name);
                        println!(" -> {from_name} => {}", target_path.display());
                        pairs.push((from_path.clone(), target_path));
                        break;
                    }
                }
            }
        }
    }

    if pairs.is_empty() {
        return Ok(());
    }

    for (from_path, target_path) in &pairs {
        println!("合并: {} -> {}", from_path.display(), target_path.display());
        move_elements_across_dir(
            from_path,
            target_path,
            MoveOptions::default(),
            &REPLACE_OPTION_UPDATE_PACK,
        )
        .await?;
    }

    Ok(())
}

/// Merge split folders back together.
///
/// This reverses the `split_folders_with_first_char` operation by moving
/// contents from single-character-named subdirectories back to the parent.
///
/// # Errors
///
/// Returns [`anyhow::Error`] if directory operations fail.
pub async fn merge_split_folders(root_dir: &Path) -> Result<(), DomainError> {
    let mut read_dir = tokio::fs::read_dir(root_dir).await?;
    let mut entries = Vec::new();
    while let Some(entry) = read_dir.next_entry().await? {
        if entry.path().is_dir() {
            entries.push(entry);
        }
    }

    let dir_names: Vec<String> = entries
        .iter()
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .collect();

    let mut pairs: Vec<(String, String)> = Vec::new();

    for dir_name in &dir_names {
        let dir_path = root_dir.join(dir_name);
        if !dir_path.is_dir() {
            continue;
        }

        if dir_name.ends_with(']') {
            let Some(dir_name_mps_i) = dir_name.rfind('[') else {
                continue;
            };
            let dir_name_without_artist = &dir_name[..dir_name_mps_i - 1];
            if dir_name_without_artist.is_empty() {
                continue;
            }

            let dir_path_without_artist = root_dir.join(dir_name_without_artist);
            if !dir_path_without_artist.is_dir() {
                continue;
            }

            let dir_names_with_starter: Vec<&String> = dir_names
                .iter()
                .filter(|d| d.starts_with(&format!("{dir_name_without_artist} [")))
                .collect();

            if dir_names_with_starter.len() > 2 {
                println!(
                    " !_! {dir_name_without_artist} have more then 2 folders! {dir_names_with_starter:?}"
                );
                continue;
            }

            pairs.push((dir_name_without_artist.to_string(), dir_name.clone()));
        }
    }

    let mut last_from_dir_name = String::new();
    let mut duplicate_list: Vec<String> = Vec::new();
    for (_target_dir_name, from_dir_name) in &pairs {
        if last_from_dir_name == *from_dir_name {
            duplicate_list.push(from_dir_name.clone());
        }
        last_from_dir_name.clone_from(from_dir_name);
    }

    if !duplicate_list.is_empty() {
        println!("Duplicate!");
        for name in &duplicate_list {
            println!(" -> {name}");
        }
        return Err(DomainError::Archive(anyhow::anyhow!(
            "Found duplicate target directories: {duplicate_list:?}"
        )));
    }

    for (target_dir_name, from_dir_name) in &pairs {
        println!("- Find Dir pair: {target_dir_name} <- {from_dir_name}");
    }

    for (target_dir_name, from_dir_name) in &pairs {
        let from_dir_path = root_dir.join(from_dir_name);
        let target_dir_path = root_dir.join(target_dir_name);
        println!(" - Moving: {target_dir_name} <- {from_dir_name}");
        move_elements_across_dir(
            &from_dir_path,
            &target_dir_path,
            MoveOptions::default(),
            &ReplaceOptions::default(),
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(prefix: &str) -> PathBuf {
        let d = std::env::temp_dir().join("bms_test_pack").join(format!(
            "{}_{}",
            prefix,
            std::process::id()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[tokio::test]
    async fn test_split_by_first_char() {
        let root = temp_dir("split_char");
        let a_dir = root.join("AlphaSong");
        let b_dir = root.join("BetaSong");
        std::fs::create_dir_all(&a_dir).unwrap();
        std::fs::create_dir_all(&b_dir).unwrap();

        split_folders_with_first_char(&root).await.unwrap();

        let parent = root.parent().unwrap();
        let abcd_dir = parent.join(format!(
            "{} [ABCD]",
            root.file_name().unwrap().to_string_lossy()
        ));
        assert!(abcd_dir.is_dir(), "ABCD group should exist");
        assert!(abcd_dir.join("AlphaSong").is_dir());
        assert!(abcd_dir.join("BetaSong").is_dir());
        let _ = std::fs::remove_dir_all(parent.join(root.file_name().unwrap()));
        let _ = std::fs::remove_dir_all(&abcd_dir);
    }

    #[tokio::test]
    async fn test_undo_split_pack() {
        let root = temp_dir("undo_split");
        let root_name = root.file_name().unwrap().to_string_lossy().to_string();
        let parent = root.parent().unwrap();
        let group_dir = parent.join(format!("{root_name} [ABCD]"));
        std::fs::create_dir_all(&group_dir).unwrap();
        std::fs::write(group_dir.join("SongName.bms"), "#TITLE Song\n").unwrap();

        undo_split_pack(&root).await.unwrap();

        assert!(
            root.join("SongName.bms").is_file(),
            "file should be moved back to root"
        );
        let _ = std::fs::remove_dir_all(parent.join(&root_name));
        let _ = std::fs::remove_dir_all(&group_dir);
    }

    #[tokio::test]
    async fn test_move_works_in_pack() {
        let src = temp_dir("move_src");
        let dst = temp_dir("move_dst");
        std::fs::create_dir_all(src.join("Song1")).unwrap();
        std::fs::write(src.join("Song1/test.bms"), "#TITLE Song1\n").unwrap();

        move_works_in_pack(&src, &dst).await.unwrap();

        assert!(dst.join("Song1/test.bms").is_file());
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&dst);
    }

    #[tokio::test]
    async fn test_move_out_works() {
        let root = temp_dir("move_out");
        let sub = root.join("SubPack");
        let work = sub.join("Work1");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("test.bms"), "#TITLE Work1\n").unwrap();

        move_out_works(&root).await.unwrap();

        assert!(
            root.join("Work1/test.bms").is_file(),
            "work should be one level up"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn test_move_works_with_same_name() {
        let src = temp_dir("same_name_src");
        let dst = temp_dir("same_name_dst");
        std::fs::create_dir_all(src.join("Song1")).unwrap();
        std::fs::write(src.join("Song1/test.bms"), "#TITLE Song1\n").unwrap();
        std::fs::create_dir_all(dst.join("Song1 [Artist]")).unwrap();
        std::fs::write(dst.join("Song1 [Artist]/readme.txt"), "info").unwrap();

        move_works_with_same_name(&src, &dst).await.unwrap();

        assert!(
            dst.join("Song1 [Artist]/test.bms").is_file(),
            "bms should be merged"
        );
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&dst);
    }

    #[tokio::test]
    async fn test_move_works_with_same_name_to_siblings() {
        let parent = temp_dir("sibling_parent");
        let src = parent.join("SourcePack");
        let sibling = parent.join("SiblingPack");
        std::fs::create_dir_all(src.join("Song1")).unwrap();
        std::fs::write(src.join("Song1/test.bms"), "#TITLE Song1\n").unwrap();
        std::fs::create_dir_all(sibling.join("Song1 [Artist]")).unwrap();

        move_works_with_same_name_to_siblings(&src).await.unwrap();

        assert!(
            sibling.join("Song1 [Artist]/test.bms").is_file(),
            "song should move to sibling"
        );
        let _ = std::fs::remove_dir_all(&parent);
    }

    #[tokio::test]
    async fn test_merge_split_folders() {
        let root = temp_dir("merge_split");
        let cat_dir = root.join("Song [Artist]");
        let base_dir = root.join("Song");
        std::fs::create_dir_all(&cat_dir).unwrap();
        std::fs::write(cat_dir.join("song.bms"), "#TITLE Song\n").unwrap();
        std::fs::create_dir_all(&base_dir).unwrap();
        std::fs::write(base_dir.join("readme.txt"), "info").unwrap();

        merge_split_folders(&root).await.unwrap();

        assert!(base_dir.join("song.bms").is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}
