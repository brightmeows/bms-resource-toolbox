//! Directory-level BMS operations.
//!
//! This module provides functions for scanning directories for BMS files
//! and extracting aggregated BMS information.

use std::path::Path;

use crate::domain::bms::encoding::{get_bms_file_str, get_boftt_encoding};
use crate::domain::bms::parse::{parse_bms_content, parse_bmson_file};
use crate::domain::bms::types::{BMS_FILE_EXTS, BMSInfo, BMSON_FILE_EXTS};
use crate::domain::bms::work::{extract_work_name, extract_work_name_for_artist};
use tokio::fs;

async fn get_dir_bms_list(dir_path: &Path) -> Vec<BMSInfo> {
    let mut info_list: Vec<BMSInfo> = Vec::new();

    let dir_name = dir_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let id = if let Some(dot_pos) = dir_name.find('.') {
        Some(&dir_name[..dot_pos])
    } else {
        Some(dir_name)
    };
    let boftt_encoding = id.and_then(get_boftt_encoding);

    let Ok(mut entries) = fs::read_dir(dir_path).await else {
        tracing::warn!("Failed to read directory: {}", dir_path.display());
        return info_list;
    };

    while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
        let file_path = entry.path();
        if !file_path.is_file() {
            continue;
        }

        let file_name = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let lower_name = file_name.to_lowercase();
        let is_bms_file = BMS_FILE_EXTS.iter().any(|ext| lower_name.ends_with(ext));
        let is_bmson_file = BMSON_FILE_EXTS.iter().any(|ext| lower_name.ends_with(ext));

        if is_bms_file
            && let Some(info) = parse_bms_file_with_encoding(&file_path, boftt_encoding).await
        {
            info_list.push(info);
        } else if is_bmson_file && let Ok(info) = parse_bmson_file(&file_path, boftt_encoding).await
        {
            info_list.push(info);
        }
    }

    info_list
}

async fn parse_bms_file_with_encoding(file_path: &Path, encoding: Option<&str>) -> Option<BMSInfo> {
    let bytes = fs::read(file_path).await.ok()?;

    let content = get_bms_file_str(&bytes, encoding);

    Some(parse_bms_content(&content))
}

/// Get aggregated `BMSInfo` for a directory.
///
/// Gets list of all BMS files in directory, extracts common title/artist/genre
/// using longest-common-prefix, returns `BMSInfo` with aggregated metadata.
pub async fn get_dir_bms_info(bms_dir_path: &Path) -> Option<BMSInfo> {
    let bms_list = get_dir_bms_list(bms_dir_path).await;
    if bms_list.is_empty() {
        return None;
    }

    let titles: Vec<String> = bms_list.iter().map(|b| b.title.clone()).collect();
    let title = extract_work_name(&titles, true, &[]);

    let title = {
        let mut result = title;
        let mut chars: Vec<char> = result.chars().collect();
        if chars.last() == Some(&'-') {
            let dash_count = chars.iter().filter(|&&c| c == '-').count();
            if dash_count % 2 != 0 && chars.len() >= 2 {
                let before_dash = chars[chars.len() - 2];
                if before_dash.is_whitespace() {
                    chars.pop();
                    result = chars.into_iter().collect();
                    result = result.trim().to_string();
                }
            }
        }
        result
    };

    let artists: Vec<String> = bms_list.iter().map(|b| b.artist.clone()).collect();
    let artist = extract_work_name_for_artist(&artists);

    let genres: Vec<String> = bms_list.iter().map(|b| b.genre.clone()).collect();
    let genre = extract_work_name(&genres, true, &[]);

    Some(BMSInfo::new(title, artist, genre))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(prefix: &str) -> PathBuf {
        let d = std::env::temp_dir().join("bms_test_dir").join(format!(
            "{}_{}",
            prefix,
            std::process::id()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[tokio::test]
    async fn test_get_dir_bms_info_basic() {
        let dir = temp_dir("info_basic");
        std::fs::write(
            dir.join("test.bms"),
            "#TITLE MySong\n#ARTIST MyArtist\n#GENRE MyGenre\n",
        )
        .unwrap();
        let info = get_dir_bms_info(&dir).await;
        assert!(info.is_some(), "should find bms info");
        let info = info.unwrap();
        assert_eq!(info.title, "MySong");
        assert_eq!(info.artist, "MyArtist");
        assert_eq!(info.genre, "MyGenre");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_get_dir_bms_info_multiple_files() {
        let dir = temp_dir("info_multi");
        std::fs::write(
            dir.join("test.bms"),
            "#TITLE CommonTitle - Another\n#ARTIST ArtistA\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("test2.bms"),
            "#TITLE CommonTitle - Hyper\n#ARTIST ArtistB\n",
        )
        .unwrap();
        let info = get_dir_bms_info(&dir).await;
        assert!(info.is_some(), "should find bms info");
        let info = info.unwrap();
        assert!(
            info.title.contains("CommonTitle"),
            "common prefix should be extracted: {}",
            info.title
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_get_dir_bms_info_no_bms_files() {
        let dir = temp_dir("info_none");
        std::fs::write(dir.join("readme.txt"), "no bms here").unwrap();
        let info = get_dir_bms_info(&dir).await;
        assert!(info.is_none(), "no bms files -> None");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn test_get_dir_bms_info_bmson() {
        let dir = temp_dir("info_bmson");
        let bmson =
            r#"{"info":{"title":"BMSON Song","artist":"BMSON Artist","genre":"BMSON Genre"}}"#;
        std::fs::write(dir.join("test.bmson"), bmson).unwrap();
        let info = get_dir_bms_info(&dir).await;
        assert!(info.is_some(), "should find bmson info");
        let info = info.unwrap();
        assert_eq!(info.title, "BMSON Song");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
