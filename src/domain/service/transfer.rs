use std::path::Path;

use crate::domain::error::DomainError;
use crate::domain::port::{FsPort, OutputPort};
use crate::infra::media::audio::{
    AUDIO_PRESET_FLAC, AUDIO_PRESET_FLAC_FFMPEG, AUDIO_PRESET_OGG_Q10, AUDIO_PRESET_WAV_FFMPEG,
    AUDIO_PRESET_WAV_FROM_FLAC,
};
use crate::infra::media::video::{
    VIDEO_PRESET_AVI_512X512, VIDEO_PRESET_MPEG1VIDEO_512X512, VIDEO_PRESET_WMV2_512X512,
    transfer_video_by_format_in_dir,
};
use crate::infra::media::{TransferOptions, transfer_audio_by_format_in_dir};

/// Available audio transfer modes for BMS work directories.
#[derive(Debug, Clone, Copy)]
pub enum AudioMode {
    /// WAV → FLAC conversion.
    WavToFlac = 0,
    /// FLAC → OGG Q10 compression.
    FlacToOgg = 1,
    /// WAV → OGG Q10 compression.
    WavToOgg = 2,
    /// FLAC → WAV reverse conversion.
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

/// Available video transfer formats for BMS work directories.
#[derive(Debug, Clone, Copy)]
pub enum VideoFormat {
    /// MP4 → AVI 512x512.
    Avi = 0,
    /// MP4 → WMV2 512x512.
    Wmv2 = 1,
    /// MP4 → MPEG1VIDEO 512x512.
    Mpeg1 = 2,
}

impl VideoFormat {
    /// All available formats.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Avi, Self::Wmv2, Self::Mpeg1]
    }
}

/// Service for bulk BMS file media transfer (audio/video conversion).
pub struct TransferService {
    fs: Box<dyn FsPort>,
    output: Box<dyn OutputPort>,
}

impl TransferService {
    /// Create a new `TransferService` with the given port implementations.
    #[must_use]
    pub fn new(fs: Box<dyn FsPort>, output: Box<dyn OutputPort>) -> Self {
        Self { fs, output }
    }

    /// Transfer audio files in `root_dir` subdirectories according to `mode`.
    ///
    /// # Errors
    ///
    /// Returns an error if directory operations fail.
    pub async fn transfer_audio(
        &self,
        root_dir: &Path,
        mode: AudioMode,
    ) -> Result<(), DomainError> {
        self.output
            .info(&format!("Audio Transfer for: {root_dir:?}"));

        let (exts, presets) = match mode {
            AudioMode::WavToFlac => (
                vec!["wav"],
                vec![AUDIO_PRESET_FLAC.clone(), AUDIO_PRESET_FLAC_FFMPEG.clone()],
            ),
            AudioMode::FlacToOgg => (vec!["flac"], vec![AUDIO_PRESET_OGG_Q10.clone()]),
            AudioMode::WavToOgg => (vec!["wav"], vec![AUDIO_PRESET_OGG_Q10.clone()]),
            AudioMode::FlacToWav => (
                vec!["flac"],
                vec![
                    AUDIO_PRESET_WAV_FROM_FLAC.clone(),
                    AUDIO_PRESET_WAV_FFMPEG.clone(),
                ],
            ),
        };

        let entries = self.fs.read_dir(root_dir).await?;
        for entry in entries {
            if !entry.is_dir {
                continue;
            }

            let bms_dir_name = entry.name.as_str();
            self.output.info(&format!("Processing: {bms_dir_name}"));

            let _ = transfer_audio_by_format_in_dir(
                &entry.path,
                &exts,
                &presets,
                &TransferOptions {
                    remove_origin_on_success: true,
                    remove_origin_on_failed: false,
                    remove_existing_target_file: true,
                    stop_on_error: true,
                },
            )
            .await;
        }

        Ok(())
    }

    /// Transfer video files in `root_dir` subdirectories to `format`.
    ///
    /// # Errors
    ///
    /// Returns an error if directory operations fail.
    pub async fn transfer_video(
        &self,
        root_dir: &Path,
        format: VideoFormat,
    ) -> Result<(), DomainError> {
        self.output
            .info(&format!("Video Transfer for: {root_dir:?}"));

        let preset = match format {
            VideoFormat::Avi => VIDEO_PRESET_AVI_512X512.clone(),
            VideoFormat::Wmv2 => VIDEO_PRESET_WMV2_512X512.clone(),
            VideoFormat::Mpeg1 => VIDEO_PRESET_MPEG1VIDEO_512X512.clone(),
        };

        let entries = self.fs.read_dir(root_dir).await?;
        for entry in entries {
            if !entry.is_dir {
                continue;
            }

            let bms_dir_name = entry.name.as_str();
            self.output.info(&format!("Processing: {bms_dir_name}"));

            transfer_video_by_format_in_dir(
                &entry.path,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::service::test_util::MockOutput;
    use crate::infra::adapters::fs::TokioFsAdapter;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_transfer_audio_empty_root() {
        let root = TempDir::new().unwrap();
        let service = TransferService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        let result = service
            .transfer_audio(root.path(), AudioMode::WavToFlac)
            .await;
        assert!(result.is_ok(), "empty root should succeed");
    }

    #[tokio::test]
    async fn test_transfer_video_empty_root() {
        let root = TempDir::new().unwrap();
        let service = TransferService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        let result = service.transfer_video(root.path(), VideoFormat::Avi).await;
        assert!(result.is_ok(), "empty root should succeed");
    }

    #[tokio::test]
    async fn test_transfer_audio_modes_all() {
        for mode in AudioMode::all() {
            let root = TempDir::new().unwrap();
            let work = root.path().join("Song");
            fs::create_dir_all(&work).await.unwrap();
            fs::write(work.join("test.wav"), "data").await.unwrap();
            let service =
                TransferService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
            let _result = service.transfer_audio(root.path(), *mode).await;
        }
    }
}
