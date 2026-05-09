use chrono::TimeZone;
use std::path::{Path, PathBuf};

use super::encode::encode_cp437;
use tokio::fs;

/// Extract an archive (zip, 7z, rar) into the output directory.
///
/// Unrecognized extensions are treated as single files and copied directly.
///
/// # Errors
///
/// Returns an error if the archive cannot be read, extracted, or if the output
/// directory cannot be created.
pub async fn extract_archive(archive_path: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let ext = archive_path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();

    fs::create_dir_all(output_dir).await?;

    match ext.as_str() {
        "zip" => {
            let archive_path = archive_path.to_path_buf();
            let output_dir = output_dir.to_path_buf();
            let file = fs::File::open(&archive_path).await?;
            let file = file.into_std().await;
            match tokio::task::spawn_blocking(move || extract_zip(file, &output_dir)).await {
                Ok(result) => result?,
                Err(e) => return Err(anyhow::anyhow!("Join error: {e}")),
            }
        }
        "7z" => extract_7z(archive_path, output_dir).await?,
        "rar" => extract_rar(archive_path, output_dir).await?,
        _ => {
            let target_path = output_dir.join(archive_path.file_name().unwrap_or_default());
            fs::copy(archive_path, &target_path).await?;
        }
    }

    Ok(())
}

fn extract_zip(file: std::fs::File, output_dir: &Path) -> Result<(), std::io::Error> {
    use zip::ZipArchive;

    let mut archive = ZipArchive::new(file)?;

    let use_cp932 = detect_cp932_encoding(&mut archive);

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let decoded_name = if let Some(enclosed) = file.enclosed_name() {
            enclosed.to_string_lossy().to_string()
        } else if use_cp932 {
            decode_cp932_filename(file.name()).unwrap_or_else(|| file.name().to_string())
        } else {
            file.name().to_string()
        };

        let Some(outpath) = safe_join(output_dir, &decoded_name) else {
            continue;
        };

        let dt = file.last_modified();

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
            set_mtime(&outpath, dt);
        } else {
            if let Some(p) = outpath.parent() {
                std::fs::create_dir_all(p)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
            set_mtime(&outpath, dt);
        }
    }

    Ok(())
}

fn detect_cp932_encoding<R: std::io::Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> bool {
    for i in 0..archive.len() {
        let Ok(file) = archive.by_index(i) else {
            continue;
        };

        let raw_name_bytes = file.name_raw();
        let name = file.name();

        if let Some(sjis_name) = try_decode_shift_jis(raw_name_bytes)
            && contains_japanese_or_cjk(&sjis_name)
        {
            return true;
        }

        if let Some(sjis_name) = decode_cp932_filename(name)
            && contains_japanese_or_cjk(&sjis_name)
        {
            return true;
        }
    }

    false
}

fn try_decode_shift_jis(bytes: &[u8]) -> Option<String> {
    use encoding_rs::SHIFT_JIS;
    let (decoded, _, had_errors) = SHIFT_JIS.decode(bytes);
    if had_errors {
        None
    } else {
        Some(decoded.to_string())
    }
}

fn decode_cp932_filename(name: &str) -> Option<String> {
    let cp437_bytes = encode_cp437(name)?;
    try_decode_shift_jis(&cp437_bytes)
}

fn contains_japanese_or_cjk(name: &str) -> bool {
    name.chars().any(|ch| {
        matches!(ch,
            '\u{3040}'..='\u{309F}' |
            '\u{30A0}'..='\u{30FF}' |
            '\u{3400}'..='\u{9FFF}' |
            '\u{F900}'..='\u{FAFF}' |
            '\u{FE30}'..='\u{FE4F}' |
            '\u{20000}'..='\u{2A6DF}'
        )
    })
}

async fn extract_7z(archive_path: &Path, output_dir: &Path) -> anyhow::Result<()> {
    let archive_path = archive_path.to_path_buf();
    let output_dir = output_dir.to_path_buf();
    let result = tokio::task::spawn_blocking(move || {
        sevenz_rust::decompress_file(&archive_path, &output_dir)
    })
    .await;
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(anyhow::anyhow!("7z error: {e}")),
        Err(e) => Err(anyhow::anyhow!("Join error: {e}")),
    }
}

async fn extract_rar(archive_path: &Path, output_dir: &Path) -> anyhow::Result<()> {
    use tokio::process::Command;

    let output = Command::new("unrar")
        .args([
            "x",
            "-o+",
            &archive_path.to_string_lossy(),
            &output_dir.to_string_lossy(),
        ])
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow::anyhow!("Failed to extract rar archive"));
    }
    Ok(())
}

fn safe_join(base: &Path, component: &str) -> Option<PathBuf> {
    let decoded = component.replace('\\', "/");
    let path = PathBuf::from(&decoded);

    if path.is_absolute() {
        return None;
    }

    let mut current = base.to_path_buf();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                current.pop();
                if !current.starts_with(base) {
                    return None;
                }
            }
            std::path::Component::Normal(name) => {
                current.push(name);
            }
            _ => return None,
        }
    }

    if !current.starts_with(base) {
        return None;
    }

    if let Some(parent) = current.parent()
        && parent.exists()
        && let (Ok(resolved), Ok(resolved_base)) = (parent.canonicalize(), base.canonicalize())
        && !resolved.starts_with(&resolved_base)
    {
        return None;
    }

    Some(current)
}

fn set_mtime(path: &Path, dt: Option<zip::DateTime>) {
    let Some(dt) = dt else { return };
    let local_dt = chrono::Local.with_ymd_and_hms(
        i32::from(dt.year()),
        u32::from(dt.month()),
        u32::from(dt.day()),
        u32::from(dt.hour()),
        u32::from(dt.minute()),
        u32::from(dt.second()),
    );
    if let Some(dt) = local_dt.single() {
        let ft = filetime::FileTime::from_unix_time(dt.timestamp(), 0);
        let _ = filetime::set_file_mtime(path, ft);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_encode_cp437_ascii() {
        assert_eq!(encode_cp437("hello.txt"), Some(b"hello.txt".to_vec()));
    }

    #[test]
    fn test_encode_cp437_non_ascii_returns_none() {
        assert!(encode_cp437("™").is_none());
    }

    #[test]
    fn test_contains_japanese() {
        assert!(contains_japanese_or_cjk("曲名"));
        assert!(contains_japanese_or_cjk("タイトル"));
        assert!(!contains_japanese_or_cjk("Title"));
        assert!(!contains_japanese_or_cjk(""));
    }

    #[test]
    fn test_try_decode_shift_jis() {
        assert_eq!(try_decode_shift_jis(b"\x82\xb1"), Some("こ".to_string()));
        assert!(try_decode_shift_jis(b"\xFF").is_none());
    }

    #[test]
    fn test_safe_join_normal() {
        let base = PathBuf::from("/tmp/test");
        assert_eq!(
            safe_join(&base, "file.txt").unwrap(),
            PathBuf::from("/tmp/test/file.txt")
        );
    }

    #[test]
    fn test_safe_join_traversal_blocked() {
        let base = PathBuf::from("/tmp/test");
        assert!(safe_join(&base, "../etc/passwd").is_none());
    }

    #[test]
    fn test_safe_join_absolute_blocked() {
        let base = PathBuf::from("/tmp/test");
        assert!(safe_join(&base, "/etc/passwd").is_none());
    }
}
