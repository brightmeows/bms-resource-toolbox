//! Interactive media commands.

use async_trait::async_trait;
use bms_res_tb_domain::error::DomainError;
use bms_res_tb_domain::folder::media;
use bms_res_tb_domain::transfer::{self, AudioMode, VideoFormat};

use crate::interactive::trait_def::InteractiveCommand;
use crate::interactive::types::{ParamDef, ParamValue};
use crate::interactive::Session;

// ── MediaAudio ──────────────────────────────────────────

/// Audio conversion with mode selection.
pub struct MediaAudio;

#[async_trait]
impl InteractiveCommand for MediaAudio {
    fn menu_name(&self) -> &'static str {
        "音频格式转换"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("BMS 根目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("MediaAudio: missing path").into_path();

        let modes = ["WAV → FLAC", "FLAC → OGG", "WAV → OGG", "FLAC → WAV"];
        let mode_map = [
            AudioMode::WavToFlac,
            AudioMode::FlacToOgg,
            AudioMode::WavToOgg,
            AudioMode::FlacToWav,
        ];

        let Ok(selection) = inquire::Select::new("选择转换模式:", modes.to_vec()).prompt() else {
            tracing::info!("已取消。");
            return Ok(());
        };

        let idx = modes.iter().position(|m| *m == selection).unwrap_or(0);
        let mode = *mode_map.get(idx).unwrap_or(&AudioMode::WavToFlac);
        transfer::transfer_audio(&path, mode).await
    }
}

// ── MediaVideo ──────────────────────────────────────────

/// Video conversion with format selection.
pub struct MediaVideo;

#[async_trait]
impl InteractiveCommand for MediaVideo {
    fn menu_name(&self) -> &'static str {
        "视频格式转换"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("BMS 根目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("MediaVideo: missing path").into_path();

        let formats = ["MP4 → AVI", "MP4 → WMV", "MP4 → MPEG"];
        let format_map = [VideoFormat::Avi, VideoFormat::Wmv2, VideoFormat::Mpeg1];

        let Ok(selection) = inquire::Select::new("选择输出格式:", formats.to_vec()).prompt() else {
            tracing::info!("已取消。");
            return Ok(());
        };

        let idx = formats.iter().position(|f| *f == selection).unwrap_or(0);
        let format = *format_map.get(idx).unwrap_or(&VideoFormat::Avi);
        transfer::transfer_video(&path, format).await
    }
}

// ── MediaRemoveUnneed ───────────────────────────────────

/// Remove redundant media files with preset selection.
pub struct MediaRemoveUnneed;

#[async_trait]
impl InteractiveCommand for MediaRemoveUnneed {
    fn menu_name(&self) -> &'static str {
        "清理冗余媒体文件"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("BMS 根目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("MediaRemoveUnneed: missing path").into_path();

        let presets = ["ORAJIA 规则", "WAV 存在时移除 FLAC", "MPG 存在时移除 WMV"];

        let Ok(selection) = inquire::Select::new("选择清理预设:", presets.to_vec()).prompt() else {
            tracing::info!("已取消。");
            return Ok(());
        };

        let rule = match selection {
            "WAV 存在时移除 FLAC" => media::get_remove_media_rule_wav_flac(),
            "MPG 存在时移除 WMV" => media::get_remove_media_rule_mpg_wmv(),
            _ => media::get_remove_media_rule_oraja(),
        };

        media::remove_unneed_media_files(&path, rule).await
    }
}
