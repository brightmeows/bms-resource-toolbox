//! Video conversion presets and processing.
//!
//! This module provides video conversion presets for formats
//! like AVI, WMV, and MPEG using ffmpeg.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::LazyLock;
use tokio::fs;
use tokio::process::Command;

/// Video conversion preset.
#[derive(Debug, Clone)]
pub struct VideoPreset {
    /// Executable name (e.g. "ffmpeg")
    pub exec: String,
    /// Input argument (e.g. "`-hide_banner` -i")
    pub input_arg: String,
    /// Filter argument (e.g. `FLITER_512X512`)
    pub filter_arg: String,
    /// Output file extension (e.g. "avi")
    pub output_file_ext: String,
    /// Output codec (e.g. "mpeg4")
    pub output_codec: String,
    /// Additional arguments (e.g. "-an -q:v 8")
    pub arg: String,
}

impl VideoPreset {
    /// Create a new video preset.
    #[must_use]
    pub fn new(
        exec: &str,
        input_arg: &str,
        filter_arg: &str,
        output_file_ext: &str,
        output_codec: &str,
        arg: &str,
    ) -> Self {
        Self {
            exec: exec.to_string(),
            input_arg: input_arg.to_string(),
            filter_arg: filter_arg.to_string(),
            output_file_ext: output_file_ext.to_string(),
            output_codec: output_codec.to_string(),
            arg: arg.to_string(),
        }
    }

    /// Get output file path by replacing extension
    #[must_use]
    pub fn get_output_file_path(&self, input_file_path: &Path) -> PathBuf {
        let stem = input_file_path.file_stem().unwrap_or_default();
        input_file_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!(
                "{}.{}",
                stem.to_string_lossy(),
                self.output_file_ext
            ))
    }

    /// Get ffmpeg command string.
    #[must_use]
    pub fn get_video_process_cmd(&self, input_file_path: &Path, output_file_path: &Path) -> String {
        let input = input_file_path.to_string_lossy();
        let output = output_file_path.to_string_lossy();
        let inner_arg = if self.exec == "ffmpeg" {
            "-map_metadata 0"
        } else {
            ""
        };
        format!(
            "{} {} \"{}\" {} {} -c:v {} {} \"{}\"",
            self.exec,
            self.input_arg,
            input,
            self.filter_arg,
            inner_arg,
            self.output_codec,
            self.arg,
            output
        )
    }
}

/// Filter complex for 512x512 with boxblur overlay
pub const FLITER_512X512: &str = "-filter_complex \"[0:v]scale=512:512:force_original_aspect_ratio=increase,crop=512:512:(ow-iw)/2:(oh-ih)/2,boxblur=20[v1];[0:v]scale=512:512:force_original_aspect_ratio=decrease[v2];[v1][v2]overlay=(main_w-overlay_w)/2:(main_h-overlay_h)/2[vid]\" -map [vid]";

/// Filter complex for 640x480 with boxblur overlay
pub const FLITER_480P: &str = "-filter_complex \"[0:v]scale=640:480:force_original_aspect_ratio=increase,crop=640:480:(ow-iw)/2:(oh-ih)/2,boxblur=20[v1];[0:v]scale=640:480:force_original_aspect_ratio=decrease[v2];[v1][v2]overlay=(main_w-overlay_w)/2:(main_h-overlay_h)/2[vid]\" -map [vid]";

/// Video preset for AVI encoding at 512x512.
#[must_use]
pub fn video_preset_avi_512x512() -> VideoPreset {
    VideoPreset::new(
        "ffmpeg",
        "-hide_banner -i",
        FLITER_512X512,
        "avi",
        "mpeg4",
        "-an -q:v 8",
    )
}

/// Video preset for MPEG1 encoding at 512x512.
#[must_use]
pub fn video_preset_mpeg1video_512x512() -> VideoPreset {
    VideoPreset::new(
        "ffmpeg",
        "-hide_banner -i",
        FLITER_512X512,
        "mpg",
        "mpeg1video",
        "-an -b:v 1500k",
    )
}

/// Video preset for WMV2 encoding at 512x512.
#[must_use]
pub fn video_preset_wmv2_512x512() -> VideoPreset {
    VideoPreset::new(
        "ffmpeg",
        "-hide_banner -i",
        FLITER_512X512,
        "wmv",
        "wmv2",
        "-an -q:v 8",
    )
}

/// Lazy static for AVI 512x512 video preset.
pub static VIDEO_PRESET_AVI_512X512: LazyLock<VideoPreset> =
    LazyLock::new(video_preset_avi_512x512);
/// Lazy static for MPEG1 512x512 video preset.
pub static VIDEO_PRESET_MPEG1VIDEO_512X512: LazyLock<VideoPreset> =
    LazyLock::new(video_preset_mpeg1video_512x512);
/// Lazy static for WMV2 512x512 video preset.
pub static VIDEO_PRESET_WMV2_512X512: LazyLock<VideoPreset> =
    LazyLock::new(video_preset_wmv2_512x512);

/// Video preset for AVI encoding at 480p.
#[must_use]
pub fn video_preset_avi_480p() -> VideoPreset {
    VideoPreset::new(
        "ffmpeg",
        "-hide_banner -i",
        FLITER_480P,
        "avi",
        "mpeg4",
        "-an -q:v 8",
    )
}

/// Video preset for WMV2 encoding at 480p.
#[must_use]
pub fn video_preset_wmv2_480p() -> VideoPreset {
    VideoPreset::new(
        "ffmpeg",
        "-hide_banner -i",
        FLITER_480P,
        "wmv",
        "wmv2",
        "-an -q:v 8",
    )
}

/// Video preset for MPEG1 encoding at 480p.
#[must_use]
pub fn video_preset_mpeg1video_480p() -> VideoPreset {
    VideoPreset::new(
        "ffmpeg",
        "-hide_banner -i",
        FLITER_480P,
        "mpg",
        "mpeg1video",
        "-an -b:v 1500k",
    )
}

/// Lazy static for AVI 480p video preset.
pub static VIDEO_PRESET_AVI_480P: LazyLock<VideoPreset> = LazyLock::new(video_preset_avi_480p);
/// Lazy static for WMV2 480p video preset.
pub static VIDEO_PRESET_WMV2_480P: LazyLock<VideoPreset> = LazyLock::new(video_preset_wmv2_480p);
/// Lazy static for MPEG1 480p video preset.
pub static VIDEO_PRESET_MPEG1VIDEO_480P: LazyLock<VideoPreset> =
    LazyLock::new(video_preset_mpeg1video_480p);

/// Get preferred video preset list based on video dimensions.
///
/// For wide aspect ratios (> 640:480), prefers 480p presets.
/// For square/aspect ratios, prefers 512x512 presets.
#[must_use]
pub fn get_prefered_preset_list(width: u32, height: u32) -> Vec<VideoPreset> {
    if f64::from(width) / f64::from(height) > 640.0 / 480.0 {
        vec![
            video_preset_mpeg1video_480p(),
            video_preset_wmv2_480p(),
            video_preset_avi_480p(),
        ]
    } else {
        vec![
            video_preset_mpeg1video_512x512(),
            video_preset_wmv2_512x512(),
            video_preset_avi_512x512(),
        ]
    }
}

/// Get video dimensions using ffprobe.
///
/// Returns `None` if ffprobe is not available, the file cannot be read,
/// or no video stream is found.
#[must_use]
pub async fn get_video_size(file_path: &Path) -> Option<(u32, u32)> {
    let cmd = format!(
        "ffprobe -v quiet -print_format json -show_streams \"{}\"",
        file_path.to_string_lossy()
    );
    let (shell, shell_arg) = if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };
    let output = Command::new(shell)
        .args([shell_arg, &cmd])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let streams = json.get("streams")?.as_array()?;
    for stream in streams {
        if stream.get("codec_type")?.as_str()? == "video" {
            let w = u32::try_from(stream.get("width")?.as_u64()?).ok()?;
            let h = u32::try_from(stream.get("height")?.as_u64()?).ok()?;
            return Some((w, h));
        }
    }
    None
}

/// Transfer video files in directory using presets (with fallback).
///
/// For each file matching `input_exts`, try each preset in order. If conversion
/// succeeds: delete original (if `remove_origin_file`), break. If conversion
/// fails: delete failed output, try next preset. Only report error when last
/// preset fails. Uses FIFO ordering (`VecDeque`) for handle completion.
/// Propagates errors from handles.
///
/// # Errors
///
/// Returns `std::io::Error` if all presets fail for any file,
/// or if a spawned task panics.
///
/// # Panics
///
/// May panic if a spawned task panics, which propagates through
/// the `JoinHandle`.
#[expect(clippy::too_many_lines)]
pub async fn transfer_video_by_format_in_dir(
    dir: &Path,
    input_exts: &[&str],
    presets: &[VideoPreset],
    remove_origin_file: bool,
    remove_existing_target_file: bool,
    use_prefered: bool,
) -> anyhow::Result<()> {
    let cpu_count = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);

    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Some(entry) = entries.next_entry().await.unwrap_or(None) {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension()
                && input_exts
                    .iter()
                    .any(|e| e.to_lowercase() == ext.to_string_lossy().to_lowercase())
            {
                files.push(path);
            }
        }
    }

    println!("Found {} video files to convert in {:?}", files.len(), dir);

    if files.is_empty() {
        return Ok(());
    }

    let mut handles: std::collections::VecDeque<tokio::task::JoinHandle<anyhow::Result<()>>> =
        std::collections::VecDeque::new();

    for file_path in files {
        while handles.len() >= cpu_count {
            if let Some(handle) = handles.pop_front() {
                handle.await??;
            }
        }

        let presets_for_this_file: Vec<VideoPreset> = if use_prefered {
            if let Some((w, h)) = get_video_size(&file_path).await {
                let mut preferred = get_prefered_preset_list(w, h);
                preferred.extend_from_slice(presets);
                preferred
            } else {
                presets.to_vec()
            }
        } else {
            presets.to_vec()
        };
        let presets_clone = presets_for_this_file;
        let handle = tokio::spawn(async move {
            let mut last_error = false;
            let mut last_err_msg = String::new();

            let presets_for_file = presets_clone;

            for (i, preset) in presets_for_file.iter().enumerate() {
                let output = preset.get_output_file_path(&file_path);

                if file_path == output {
                    break;
                }

                if output.is_file() {
                    if remove_existing_target_file {
                        let _ = fs::remove_file(&output).await;
                    } else {
                        println!("File exists: {output:?}");
                        continue;
                    }
                }

                let cmd_str = preset.get_video_process_cmd(&file_path, &output);
                tracing::info!("Running: {}", cmd_str);

                let (shell, shell_arg) = if std::env::consts::OS == "windows" {
                    ("cmd", "/C")
                } else {
                    ("sh", "-c")
                };

                let result = Command::new(shell)
                    .args([shell_arg, &cmd_str])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await;

                match result {
                    Ok(output_result) if output_result.status.success() => {
                        if remove_origin_file && file_path.is_file() {
                            let _ = fs::remove_file(&file_path).await;
                        }
                        break;
                    }
                    Ok(output_result) => {
                        let stdout = String::from_utf8_lossy(&output_result.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output_result.stderr).to_string();
                        if output.is_file() {
                            let _ = fs::remove_file(&output).await;
                        }
                        if i == presets_for_file.len() - 1 {
                            last_error = true;
                            last_err_msg = format!(
                                "Conversion failed\nCmd: {cmd_str}\nStdout: {stdout}\nStderr: {stderr}"
                            );
                        }
                    }
                    Err(e) => {
                        if output.is_file() {
                            let _ = fs::remove_file(&output).await;
                        }
                        if i == presets_for_file.len() - 1 {
                            last_error = true;
                            last_err_msg = format!("Conversion failed: {e}");
                        }
                    }
                }
            }

            if last_error {
                println!("Has Error!");
                println!("{last_err_msg}");
                Err(anyhow::anyhow!(
                    "All presets failed for {}",
                    file_path.display()
                ))
            } else {
                Ok(())
            }
        });
        handles.push_back(handle);
    }

    for handle in handles {
        handle.await??;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_preset_cmd_matches_python() {
        let preset = video_preset_avi_512x512();
        let input = Path::new("/path/to/input.mp4");
        let output = Path::new("/path/to/output.avi");
        let cmd = preset.get_video_process_cmd(input, output);
        assert!(cmd.contains("ffmpeg -hide_banner -i"));
        assert!(cmd.contains("input.mp4"));
        assert!(cmd.contains("output.avi"));
        assert!(cmd.contains("-c:v mpeg4"));
        assert!(cmd.contains("-an -q:v 8"));
        assert!(cmd.contains("-map_metadata 0"));
        assert!(cmd.contains("-filter_complex"));
        assert!(!cmd.contains("-vf"));
    }

    #[test]
    fn test_video_preset_avi() {
        let preset = VIDEO_PRESET_AVI_512X512.clone();
        assert_eq!(preset.output_file_ext, "avi");
        assert_eq!(preset.output_codec, "mpeg4");
    }

    #[test]
    fn test_output_file_path() {
        let preset = video_preset_avi_512x512();
        let input = Path::new("/some/dir/video.mp4");
        let output = preset.get_output_file_path(input);
        assert_eq!(output, PathBuf::from("/some/dir/video.avi"));
    }

    #[test]
    fn test_video_preset_480p_cmd() {
        let preset = video_preset_avi_480p();
        let input = Path::new("/path/to/input.mp4");
        let output = Path::new("/path/to/output.avi");
        let cmd = preset.get_video_process_cmd(input, output);
        assert!(cmd.contains("640:480"));
        assert!(cmd.contains("crop=640:480"));
    }

    #[test]
    fn test_video_preset_480p_ext() {
        let avi = video_preset_avi_480p();
        assert_eq!(avi.output_file_ext, "avi");
        let wmv = video_preset_wmv2_480p();
        assert_eq!(wmv.output_file_ext, "wmv");
        let mpg = video_preset_mpeg1video_480p();
        assert_eq!(mpg.output_file_ext, "mpg");
    }

    #[test]
    fn test_get_prefered_preset_wide() {
        let presets = get_prefered_preset_list(1920, 1080);
        assert_eq!(presets.len(), 3);
        assert!(presets[0].output_file_ext == "mpg");
    }

    #[test]
    fn test_get_prefered_preset_square() {
        let presets = get_prefered_preset_list(800, 600);
        assert_eq!(presets.len(), 3);
        assert!(presets[0].output_file_ext == "mpg");
    }

    #[test]
    fn test_parse_video_size_from_ffprobe_json() {
        let json = r#"{
            "streams": [
                {"codec_type": "audio"},
                {"codec_type": "video", "width": 1920, "height": 1080}
            ]
        }"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let streams = value.get("streams").and_then(|v| v.as_array()).unwrap();
        for stream in streams {
            if stream.get("codec_type").and_then(|v| v.as_str()) == Some("video") {
                #[expect(clippy::cast_possible_truncation)]
                let w = stream
                    .get("width")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap() as u32;
                #[expect(clippy::cast_possible_truncation)]
                let h = stream
                    .get("height")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap() as u32;
                assert_eq!((w, h), (1920, 1080));
                return;
            }
        }
        panic!("No video stream found");
    }

    #[test]
    fn test_parse_video_size_no_video_stream() {
        let json = r#"{"streams": [{"codec_type": "audio"}]}"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let streams = value.get("streams").and_then(|v| v.as_array()).unwrap();
        let video = streams
            .iter()
            .find(|s| s.get("codec_type").and_then(|v| v.as_str()) == Some("video"));
        assert!(video.is_none());
    }
}
