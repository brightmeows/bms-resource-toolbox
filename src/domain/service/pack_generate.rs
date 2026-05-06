use std::path::Path;

use crate::domain::error::DomainError;
use crate::domain::port::{FsPort, OutputPort};
use crate::infra::adapters::fs::TokioFsAdapter;
use crate::infra::adapters::output::ConsoleOutput;
use crate::infra::fs::pack_move::is_dir_having_file;
use crate::infra::fs::sync::{SYNC_PRESET_FOR_APPEND, sync_folder};
use crate::infra::fs::walk::remove_empty_dirs;
use crate::infra::media::audio::{
    AUDIO_PRESET_FLAC, AUDIO_PRESET_FLAC_FFMPEG, AUDIO_PRESET_OGG_FFMPEG, AUDIO_PRESET_OGG_Q10,
};
use crate::infra::media::video::{
    VIDEO_PRESET_AVI_512X512, VIDEO_PRESET_MPEG1VIDEO_512X512, VIDEO_PRESET_WMV2_512X512,
    transfer_video_by_format_in_dir,
};
use crate::infra::media::{TransferOptions, transfer_audio_by_format_in_dir};

/// Service for generating BMS packs: RAW→HQ and HQ→LQ conversion pipelines.
pub struct PackGenerateService {
    fs: Box<dyn FsPort>,
    output: Box<dyn OutputPort>,
}

impl PackGenerateService {
    /// Create a new `PackGenerateService` with the given port implementations.
    #[must_use]
    pub fn new(fs: Box<dyn FsPort>, output: Box<dyn OutputPort>) -> Self {
        Self { fs, output }
    }

    /// Full setup pipeline: extract numeric archives → rename dirs → WAV→FLAC → clean.
    ///
    /// # Errors
    ///
    /// Returns an error if any step fails.
    pub async fn pack_setup_rawpack_to_hq(
        &self,
        pack: &Path,
        root: &Path,
    ) -> Result<(), DomainError> {
        self.output
            .info(&format!("Pack Setup RAW -> HQ: {pack:?} -> {root:?}"));

        if !self.fs.is_dir(pack).await {
            self.output.info("Pack dir is not vaild dir.");
            return Err(DomainError::Archive(anyhow::anyhow!(
                "Pack dir is not a valid directory"
            )));
        }
        if self.fs.exists(root).await {
            return Err(DomainError::Archive(anyhow::anyhow!(
                "Directory {} already exists",
                root.display()
            )));
        }
        self.fs.create_dir_all(root).await?;
        let cache_dir = root.join("CacheDir");

        self.output
            .info(&format!("Unzipping packs from {pack:?} to {root:?}"));
        self.call_unzip_numeric(pack, &cache_dir, root).await?;

        if !is_dir_having_file(&cache_dir).await {
            self.fs.remove_dir(&cache_dir).await?;
        }

        self.output.info("Setting dir names from BMS Files");
        self.call_append_name_by_bms(root).await?;

        self.output.info("Parsing Audio... Phase 1: WAV -> FLAC");
        let flac_preset = AUDIO_PRESET_FLAC.clone();
        let flac_ffmpeg_preset = AUDIO_PRESET_FLAC_FFMPEG.clone();
        self.bms_folder_transfer_audio(
            root,
            &["wav"],
            &[flac_preset, flac_ffmpeg_preset],
            &TransferOptions {
                remove_origin_on_success: true,
                remove_origin_on_failed: true,
                remove_existing_target_file: true,
                stop_on_error: false,
            },
        )
        .await?;

        self.output.info("Removing Unneed Files");
        self.call_remove_unneed_media_files(root).await?;

        Ok(())
    }

    /// Update pipeline with sync from existing directory: extract → sync names → convert → clean → sync files.
    ///
    /// # Errors
    ///
    /// Returns an error if any step fails.
    pub async fn pack_update_rawpack_to_hq(
        &self,
        pack: &Path,
        root: &Path,
        sync: &Path,
    ) -> Result<(), DomainError> {
        self.output.info(&format!(
            "Pack Update RAW -> HQ: {pack:?} -> {root:?} (sync from {sync:?})"
        ));

        if !self.fs.is_dir(pack).await {
            self.output.info("Pack dir is not vaild dir.");
            return Err(DomainError::Archive(anyhow::anyhow!(
                "Pack dir is not a valid directory"
            )));
        }
        if self.fs.exists(root).await {
            return Err(DomainError::Archive(anyhow::anyhow!(
                "Directory {} already exists",
                root.display()
            )));
        }
        if !self.fs.is_dir(sync).await {
            self.output.info("Syncing dir is not vaild dir.");
            return Err(DomainError::Archive(anyhow::anyhow!(
                "Sync dir is not a valid directory"
            )));
        }
        self.fs.create_dir_all(root).await?;
        let cache_dir = root.join("CacheDir");

        self.output
            .info(&format!("Unzipping packs from {pack:?} to {root:?}"));
        self.call_unzip_numeric(pack, &cache_dir, root).await?;

        self.output
            .info(&format!("Syncing dir name from {sync:?} to {root:?}"));
        self.call_copy_numbered_workdir_names(sync, root).await?;

        self.output.info("Parsing Audio... Phase 1: WAV -> FLAC");
        let flac_preset = AUDIO_PRESET_FLAC.clone();
        let flac_ffmpeg_preset = AUDIO_PRESET_FLAC_FFMPEG.clone();
        self.bms_folder_transfer_audio(
            root,
            &["wav"],
            &[flac_preset, flac_ffmpeg_preset],
            &TransferOptions {
                remove_origin_on_success: true,
                remove_origin_on_failed: true,
                remove_existing_target_file: true,
                stop_on_error: false,
            },
        )
        .await?;

        self.output.info("Removing Unneed Files");
        self.call_remove_unneed_media_files(root).await?;

        self.output
            .info(&format!("Syncing dir files from {root:?} to {sync:?}"));
        sync_folder(root, sync, &SYNC_PRESET_FOR_APPEND, 8).await?;

        self.output
            .info(&format!("Removing empty folder in {root:?}"));
        remove_empty_dirs(root).await?;

        Ok(())
    }

    /// Convert a raw BMS pack to HQ: WAV→FLAC → clean unnecessary media.
    ///
    /// # Errors
    ///
    /// Returns an error if any step fails.
    pub async fn pack_raw_to_hq(&self, path: &Path) -> Result<(), DomainError> {
        self.output.info(&format!("Pack RAW -> HQ for: {path:?}"));
        self.output.info("Parsing Audio... Phase 1: WAV -> FLAC");
        let flac_preset = AUDIO_PRESET_FLAC.clone();
        let flac_ffmpeg_preset = AUDIO_PRESET_FLAC_FFMPEG.clone();
        self.bms_folder_transfer_audio(
            path,
            &["wav"],
            &[flac_preset, flac_ffmpeg_preset],
            &TransferOptions {
                remove_origin_on_success: true,
                remove_origin_on_failed: true,
                remove_existing_target_file: true,
                stop_on_error: false,
            },
        )
        .await?;

        self.output.info("Removing Unneed Files");
        self.call_remove_unneed_media_files(path).await?;

        Ok(())
    }

    /// Convert a HQ BMS pack to LQ: FLAC→OGG → MP4→AVI/WMV/MPEG.
    ///
    /// # Errors
    ///
    /// Returns an error if any step fails.
    pub async fn pack_hq_to_lq(&self, path: &Path) -> Result<(), DomainError> {
        self.output.info(&format!("Pack HQ -> LQ for: {path:?}"));
        self.output.info("Parsing Audio... Phase 1: FLAC -> OGG");
        let ogg_preset = AUDIO_PRESET_OGG_Q10.clone();
        let ogg_ffmpeg = AUDIO_PRESET_OGG_FFMPEG.clone();
        self.bms_folder_transfer_audio(
            path,
            &["flac"],
            &[ogg_preset, ogg_ffmpeg],
            &TransferOptions {
                remove_origin_on_success: true,
                remove_origin_on_failed: false,
                remove_existing_target_file: true,
                stop_on_error: false,
            },
        )
        .await?;

        self.output.info("Parsing Video...");
        let presets = vec![
            VIDEO_PRESET_MPEG1VIDEO_512X512.clone(),
            VIDEO_PRESET_WMV2_512X512.clone(),
            VIDEO_PRESET_AVI_512X512.clone(),
        ];
        self.bms_folder_transfer_video(path, &["mp4"], &presets, true, true)
            .await?;

        Ok(())
    }

    /// Call `unzip_numeric_to_bms_folder` via temporary `UnzipService`.
    async fn call_unzip_numeric(
        &self,
        pack: &Path,
        cache: &Path,
        root: &Path,
    ) -> Result<(), DomainError> {
        let svc = crate::domain::service::unzip::UnzipService::new(
            Box::new(TokioFsAdapter),
            Box::new(ConsoleOutput),
        );
        svc.unzip_numeric_to_bms_folder(pack, cache, root).await
    }

    /// Call `append_name_by_bms` via temporary `FolderRenameService`.
    async fn call_append_name_by_bms(&self, root: &Path) -> Result<(), DomainError> {
        let svc = crate::domain::service::folder_rename::FolderRenameService::new(
            Box::new(TokioFsAdapter),
            Box::new(ConsoleOutput),
        );
        svc.append_name_by_bms(root).await
    }

    /// Call `remove_unneed_media_files` via temporary `FolderMediaService`.
    async fn call_remove_unneed_media_files(&self, root: &Path) -> Result<(), DomainError> {
        let svc = crate::domain::service::folder_media::FolderMediaService::new(
            Box::new(TokioFsAdapter),
            Box::new(ConsoleOutput),
        );
        svc.remove_unneed_media_files(
            root,
            crate::domain::service::folder_media::get_remove_media_rule_oraja(),
        )
        .await
    }

    /// Call `copy_numbered_workdir_names` via temporary `FolderCleanupService`.
    async fn call_copy_numbered_workdir_names(
        &self,
        from: &Path,
        to: &Path,
    ) -> Result<(), DomainError> {
        let svc = crate::domain::service::folder_cleanup::FolderCleanupService::new(
            Box::new(TokioFsAdapter),
            Box::new(ConsoleOutput),
        );
        svc.copy_numbered_workdir_names(from, to).await
    }

    /// Iterate over subdirectories and transfer audio according to presets.
    async fn bms_folder_transfer_audio(
        &self,
        root_dir: &Path,
        input_exts: &[&str],
        presets: &[crate::infra::media::audio::AudioPreset],
        options: &TransferOptions,
    ) -> Result<(), DomainError> {
        let entries = self.fs.read_dir(root_dir).await?;
        for entry in entries {
            if !entry.is_dir {
                continue;
            }
            let bms_dir_path = entry.path;
            if let Err(e) =
                transfer_audio_by_format_in_dir(&bms_dir_path, input_exts, presets, options).await
            {
                self.output
                    .info(&format!(" - Dir: {bms_dir_path:?} Error occured!"));
                if options.stop_on_error {
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }

    /// Iterate over subdirectories and transfer video according to presets.
    async fn bms_folder_transfer_video(
        &self,
        root_dir: &Path,
        input_exts: &[&str],
        presets: &[crate::infra::media::video::VideoPreset],
        remove_origin_file: bool,
        remove_existing_target_file: bool,
    ) -> Result<(), DomainError> {
        let entries = self.fs.read_dir(root_dir).await?;
        for entry in entries {
            if !entry.is_dir {
                continue;
            }
            let bms_dir_path = entry.path;
            if let Err(e) = transfer_video_by_format_in_dir(
                &bms_dir_path,
                input_exts,
                presets,
                remove_origin_file,
                remove_existing_target_file,
                false,
            )
            .await
            {
                self.output.info("Error occured!");
                return Err(e.into());
            }
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
    async fn test_pack_raw_to_hq_does_not_panic() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("TestSong");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(work.join("test.wav"), "fake-wav-data")
            .await
            .unwrap();
        let service =
            PackGenerateService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        let _result = service.pack_raw_to_hq(root.path()).await;
    }

    #[tokio::test]
    async fn test_pack_hq_to_lq_does_not_panic() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("TestSong");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(work.join("test.flac"), "fake-flac-data")
            .await
            .unwrap();
        fs::write(work.join("test.mp4"), "fake-mp4-data")
            .await
            .unwrap();
        let service =
            PackGenerateService::new(Box::new(TokioFsAdapter), Box::new(MockOutput::new()));
        let _result = service.pack_hq_to_lq(root.path()).await;
    }
}
