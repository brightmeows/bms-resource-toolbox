//! BMS folder media transfer utilities.
//!
//! This module provides interactive functions for transferring
//! audio and video files in BMS directories.

use std::path::Path;
use tokio::fs;

use crate::error::DomainError;
use bms_res_tb_infra::media::audio::{
    AUDIO_PRESET_FLAC, AUDIO_PRESET_FLAC_FFMPEG, AUDIO_PRESET_OGG_Q10, AUDIO_PRESET_WAV_FFMPEG,
    AUDIO_PRESET_WAV_FROM_FLAC, AudioPreset,
};
use bms_res_tb_infra::media::convert::{
    OriginRemoval, TransferOptions, transfer_audio_by_format_in_dir,
};
use bms_res_tb_infra::media::video::{
    VIDEO_PRESET_AVI_512X512, VIDEO_PRESET_MPEG1VIDEO_512X512, VIDEO_PRESET_WMV2_512X512,
    VideoPreset, transfer_video_by_format_in_dir,
};

/// Available audio transfer modes.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum AudioMode {
    /// WAV → FLAC
    WavToFlac = 0,
    /// FLAC → OGG Q10
    FlacToOgg = 1,
    /// WAV → OGG Q10
    WavToOgg = 2,
    /// FLAC → WAV
    FlacToWav = 3,
}

impl AudioMode {
    /// All available modes.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::WavToFlac,
            Self::FlacToOgg,
            Self::WavToOgg,
            Self::FlacToWav,
        ]
    }
}

/// Transfer audio files in a BMS root directory
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn transfer_audio(root_dir: &Path, mode: AudioMode) -> Result<(), DomainError> {
    tracing::info!("Audio Transfer for: {root_dir:?}");

    let modes_data: [(&str, Vec<&str>, Vec<AudioPreset>); 4] = [
        (
            "Convert: WAV to FLAC",
            vec!["wav"],
            vec![AUDIO_PRESET_FLAC.clone(), AUDIO_PRESET_FLAC_FFMPEG.clone()],
        ),
        (
            "Compress: FLAC to OGG Q10",
            vec!["flac"],
            vec![AUDIO_PRESET_OGG_Q10.clone()],
        ),
        (
            "Compress: WAV to OGG Q10",
            vec!["wav"],
            vec![AUDIO_PRESET_OGG_Q10.clone()],
        ),
        (
            "Reverse: FLAC to WAV",
            vec!["flac"],
            vec![
                AUDIO_PRESET_WAV_FROM_FLAC.clone(),
                AUDIO_PRESET_WAV_FFMPEG.clone(),
            ],
        ),
    ];

    let idx = mode as usize;
    #[expect(
        clippy::indexing_slicing,
        reason = "idx is from AudioMode enum, modes_data.len() == 4"
    )]
    let (_, exts, presets) = &modes_data[idx];
    let combined_exts: Vec<&str> = exts.clone();
    let combined_presets: Vec<AudioPreset> = presets.clone();

    let mut read_dir = fs::read_dir(root_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let bms_dir = entry.path();
        if !bms_dir.is_dir() {
            continue;
        }

        let bms_dir_name = bms_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

        tracing::info!("Processing: {bms_dir_name}");

        let _ = transfer_audio_by_format_in_dir(
            &bms_dir,
            &combined_exts,
            &combined_presets,
            &TransferOptions {
                origin_removal: OriginRemoval::OnSuccess,
                remove_existing_target_file: true,
                stop_on_error: true,
            },
        )
        .await;
    }

    Ok(())
}

/// Available video transfer formats.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum VideoFormat {
    /// MP4 → AVI 512x512
    Avi = 0,
    /// MP4 → WMV2 512x512
    Wmv2 = 1,
    /// MP4 → MPEG1VIDEO 512x512
    Mpeg1 = 2,
}

impl VideoFormat {
    /// All available formats.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Avi, Self::Wmv2, Self::Mpeg1]
    }
}

/// Transfer video files in a BMS root directory
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn transfer_video(root_dir: &Path, format: VideoFormat) -> Result<(), DomainError> {
    tracing::info!("Video Transfer for: {root_dir:?}");

    let presets: [(&str, VideoPreset); 3] = [
        ("MP4 -> AVI 512x512", VIDEO_PRESET_AVI_512X512.clone()),
        ("MP4 -> WMV2 512x512", VIDEO_PRESET_WMV2_512X512.clone()),
        (
            "MP4 -> MPEG1VIDEO 512x512",
            VIDEO_PRESET_MPEG1VIDEO_512X512.clone(),
        ),
    ];

    let idx = format as usize;
    #[expect(
        clippy::indexing_slicing,
        reason = "idx is from VideoFormat enum, presets.len() == 3"
    )]
    let preset = presets[idx].1.clone();

    // Process each work directory
    let mut read_dir = fs::read_dir(root_dir).await?;
    while let Some(entry) = read_dir.next_entry().await? {
        let bms_dir = entry.path();
        if !bms_dir.is_dir() {
            continue;
        }

        let bms_dir_name = bms_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");

        tracing::info!("Processing: {bms_dir_name}");

        transfer_video_by_format_in_dir(
            &bms_dir,
            &["mp4", "mkv", "avi", "wmv", "mpg", "mpeg"],
            std::slice::from_ref(&preset),
            true,
            true,
            false,
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_transfer_audio_empty_root() {
        let root = TempDir::new().unwrap();
        let result = transfer_audio(root.path(), AudioMode::WavToFlac).await;
        assert!(result.is_ok(), "empty root should succeed");
    }

    #[tokio::test]
    async fn test_transfer_video_empty_root() {
        let root = TempDir::new().unwrap();
        let result = transfer_video(root.path(), VideoFormat::Avi).await;
        assert!(result.is_ok(), "empty root should succeed");
    }

    #[tokio::test]
    async fn test_transfer_audio_modes_all() {
        for mode in AudioMode::all() {
            let root = TempDir::new().unwrap();
            let work = root.path().join("Song");
            fs::create_dir_all(&work).await.unwrap();
            fs::write(work.join("test.wav"), "data").await.unwrap();
            let _result = transfer_audio(root.path(), *mode).await;
        }
    }
}
