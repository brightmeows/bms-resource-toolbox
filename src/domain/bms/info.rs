//! BMS directory info extraction — reads BMS files via `FsPort` and produces
//! aggregated `BMSInfo` (title/artist/genre) for a directory.

use std::path::Path;

use crate::domain::bms::encoding::{get_bms_file_str, get_boftt_encoding};
use crate::domain::bms::parse::{parse_bms_content, parse_bmson_file};
use crate::domain::bms::types::{BMS_FILE_EXTS, BMSInfo, BMSON_FILE_EXTS};
use crate::domain::bms::work::{extract_work_name, extract_work_name_for_artist};
use crate::domain::port::FsPort;

async fn get_dir_bms_list(fs: &dyn FsPort, dir_path: &Path) -> Vec<BMSInfo> {
    let mut info_list: Vec<BMSInfo> = Vec::new();

    let dir_name = dir_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let id = if let Some(dot_pos) = dir_name.find('.') {
        Some(&dir_name[..dot_pos])
    } else {
        Some(dir_name)
    };
    let boftt_encoding = id.and_then(get_boftt_encoding);

    let Ok(entries) = fs.read_dir(dir_path).await else {
        tracing::warn!("Failed to read directory: {}", dir_path.display());
        return info_list;
    };

    for entry in &entries {
        if !entry.is_file {
            continue;
        }

        let lower_name = entry.name.to_lowercase();
        let is_bms_file = BMS_FILE_EXTS.iter().any(|ext| lower_name.ends_with(ext));
        let is_bmson_file = BMSON_FILE_EXTS.iter().any(|ext| lower_name.ends_with(ext));

        if is_bms_file
            && let Some(info) = parse_bms_file_with_encoding(fs, &entry.path, boftt_encoding).await
        {
            info_list.push(info);
        } else if is_bmson_file
            && let Ok(info) = parse_bmson_file(&entry.path, boftt_encoding).await
        {
            info_list.push(info);
        }
    }

    info_list
}

async fn parse_bms_file_with_encoding(
    fs: &dyn FsPort,
    file_path: &Path,
    encoding: Option<&str>,
) -> Option<BMSInfo> {
    let bytes = fs.read_file(file_path).await.ok()?;
    let content = get_bms_file_str(&bytes, encoding);
    Some(parse_bms_content(&content))
}

/// Get aggregated `BMSInfo` for a directory.
///
/// Scans all BMS/BMSON files in the directory, extracts common title/artist/genre
/// using longest-common-prefix, returns `BMSInfo` with aggregated metadata.
pub async fn get_dir_bms_info(fs: &dyn FsPort, bms_dir_path: &Path) -> Option<BMSInfo> {
    let bms_list = get_dir_bms_list(fs, bms_dir_path).await;
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
