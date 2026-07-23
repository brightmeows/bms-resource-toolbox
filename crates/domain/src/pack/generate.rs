//! Pack generation functions.
//!
//! This module provides high-level functions for generating
//! BMS packs including RAW to HQ and HQ to LQ conversion.

use super::unzip_numeric::unzip_numeric_to_bms_folder;
use tokio::fs;

use crate::error::DomainError;
use crate::folder::cleanup::copy_numbered_workdir_names;
use crate::folder::media::{get_remove_media_rule_oraja, remove_unneed_media_files};
use crate::folder::rename::append_name_by_bms;
use crate::sync::{SYNC_PRESET_FOR_APPEND, sync_folder};
use bms_res_tb_infra::fs::pack_move::is_dir_having_file;
use bms_res_tb_infra::fs::walk::remove_empty_dirs;
use bms_res_tb_infra::media::audio::{
    AUDIO_PRESET_FLAC, AUDIO_PRESET_FLAC_FFMPEG, AUDIO_PRESET_OGG_FFMPEG, AUDIO_PRESET_OGG_Q10,
};
use bms_res_tb_infra::media::convert::{
    OriginRemoval, TransferOptions, transfer_audio_by_format_in_dir,
};
use bms_res_tb_infra::media::video::{
    VIDEO_PRESET_AVI_512X512, VIDEO_PRESET_MPEG1VIDEO_512X512, VIDEO_PRESET_WMV2_512X512,
    transfer_video_by_format_in_dir,
};
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;

async fn bms_folder_transfer_audio(
    root_dir: &Path,
    input_exts: &[&str],
    presets: &[bms_res_tb_infra::media::audio::AudioPreset],
    options: &TransferOptions,
) -> Result<(), DomainError> {
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
        let exts_owned: Vec<String> = input_exts.iter().map(ToString::to_string).collect();
        let presets_vec = presets.to_vec();
        let options_clone = options.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.expect("semaphore not closed");
            let exts_refs: Vec<&str> = exts_owned.iter().map(String::as_str).collect();
            transfer_audio_by_format_in_dir(&dir_path, &exts_refs, &presets_vec, &options_clone)
                .await
                .map_err(|e| (dir_path, anyhow::anyhow!("{e}")))
        }));
    }

    let mut errors: Vec<(PathBuf, anyhow::Error)> = Vec::new();
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

    if options.stop_on_error
        && let Some((_, e)) = errors.into_iter().next()
    {
        return Err(e.into());
    }

    Ok(())
}

async fn bms_folder_transfer_video(
    root_dir: &Path,
    input_exts: &[&str],
    presets: &[bms_res_tb_infra::media::video::VideoPreset],
    remove_origin_file: bool,
    remove_existing_target_file: bool,
) -> Result<(), DomainError> {
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
        let exts_owned: Vec<String> = input_exts.iter().map(ToString::to_string).collect();
        let presets_vec = presets.to_vec();

        handles.push(tokio::spawn(async move {
            let _permit = sem_clone.acquire().await.expect("semaphore not closed");
            let exts_refs: Vec<&str> = exts_owned.iter().map(String::as_str).collect();
            transfer_video_by_format_in_dir(
                &dir_path,
                &exts_refs,
                &presets_vec,
                remove_origin_file,
                remove_existing_target_file,
                false,
            )
            .await
            .map_err(|e| (dir_path, anyhow::anyhow!("{e}")))
        }));
    }

    let mut errors: Vec<(PathBuf, anyhow::Error)> = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err((dir, e))) => {
                tracing::info!("Error occured in Dir: {dir:?}");
                errors.push((dir, e));
            }
            Err(e) => {
                return Err(std::io::Error::other(format!("Task join error: {e}")).into());
            }
        }
    }

    if let Some((_, e)) = errors.into_iter().next() {
        return Err(e.into());
    }

    Ok(())
}

/// Pack raw BMS to HQ version (for beatoraja/Qwilight)
///
/// 1. Convert WAV -> FLAC
/// 2. Remove unnecessary media files
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn pack_raw_to_hq(root_dir: &Path) -> Result<(), DomainError> {
    tracing::info!("Pack RAW -> HQ for: {root_dir:?}");

    // Phase 1: Convert WAV to FLAC
    tracing::info!("Parsing Audio... Phase 1: WAV -> FLAC");
    let flac_preset = AUDIO_PRESET_FLAC.clone();
    let flac_ffmpeg_preset = AUDIO_PRESET_FLAC_FFMPEG.clone();
    bms_folder_transfer_audio(
        root_dir,
        &["wav"],
        &[flac_preset, flac_ffmpeg_preset],
        &TransferOptions {
            origin_removal: OriginRemoval::Always,
            remove_existing_target_file: true,
            stop_on_error: false,
        },
    )
    .await?;

    // Phase 2: Remove unnecessary media files
    tracing::info!("Removing Unneed Files");
    remove_unneed_media_files(root_dir, get_remove_media_rule_oraja()).await?;

    Ok(())
}

/// Pack HQ BMS to LQ version (for LR2)
///
/// 1. Convert FLAC -> OGG
/// 2. Convert MP4 -> AVI/WMV/MPEG (512x512)
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn pack_hq_to_lq(root_dir: &Path) -> Result<(), DomainError> {
    tracing::info!("Pack HQ -> LQ for: {root_dir:?}");

    // Phase 1: Convert FLAC to OGG
    tracing::info!("Parsing Audio... Phase 1: FLAC -> OGG");
    let ogg_preset = AUDIO_PRESET_OGG_Q10.clone();
    let ogg_ffmpeg = AUDIO_PRESET_OGG_FFMPEG.clone();
    bms_folder_transfer_audio(
        root_dir,
        &["flac"],
        &[ogg_preset, ogg_ffmpeg],
        &TransferOptions {
            origin_removal: OriginRemoval::OnSuccess,
            remove_existing_target_file: true,
            stop_on_error: false,
        },
    )
    .await?;

    // Phase 2: Convert video
    tracing::info!("Parsing Video...");
    let presets = vec![
        VIDEO_PRESET_MPEG1VIDEO_512X512.clone(),
        VIDEO_PRESET_WMV2_512X512.clone(),
        VIDEO_PRESET_AVI_512X512.clone(),
    ];
    bms_folder_transfer_video(root_dir, &["mp4"], &presets, true, true).await?;

    Ok(())
}

/// Setup raw pack to HQ: extract -> rename -> convert -> clean
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn pack_setup_rawpack_to_hq(pack_dir: &Path, root_dir: &Path) -> Result<(), DomainError> {
    tracing::info!("Pack Setup RAW -> HQ: {pack_dir:?} -> {root_dir:?}");

    if !pack_dir.is_dir() {
        tracing::info!("Pack dir is not vaild dir.");
        return Err(DomainError::Archive(anyhow::anyhow!(
            "Pack dir is not a valid directory"
        )));
    }
    if root_dir.exists() {
        return Err(DomainError::Archive(anyhow::anyhow!(
            "Directory {} already exists",
            root_dir.display()
        )));
    }
    fs::create_dir_all(root_dir).await?;
    let cache_dir = root_dir.join("CacheDir");

    // Step 1: Unzip packs
    tracing::info!("Unzipping packs from {pack_dir:?} to {root_dir:?}");
    unzip_numeric_to_bms_folder(pack_dir, &cache_dir, root_dir).await?;

    // Remove cache dir if empty
    if !is_dir_having_file(&cache_dir).await {
        fs::remove_dir(&cache_dir).await?;
    }

    // Step 2: Set dir names from BMS files
    tracing::info!("Setting dir names from BMS Files");
    append_name_by_bms(root_dir).await?;

    // Step 3: Convert WAV -> FLAC
    tracing::info!("Parsing Audio... Phase 1: WAV -> FLAC");
    let flac_preset = AUDIO_PRESET_FLAC.clone();
    let flac_ffmpeg_preset = AUDIO_PRESET_FLAC_FFMPEG.clone();
    bms_folder_transfer_audio(
        root_dir,
        &["wav"],
        &[flac_preset, flac_ffmpeg_preset],
        &TransferOptions {
            origin_removal: OriginRemoval::Always,
            remove_existing_target_file: true,
            stop_on_error: false,
        },
    )
    .await?;

    // Step 4: Remove unnecessary media files
    tracing::info!("Removing Unneed Files");
    remove_unneed_media_files(root_dir, get_remove_media_rule_oraja()).await?;

    Ok(())
}

/// Update raw pack to HQ with sync from existing directory
///
/// # Errors
///
/// Returns an error if directory operations fail.
pub async fn pack_update_rawpack_to_hq(
    pack_dir: &Path,
    root_dir: &Path,
    sync_dir: &Path,
) -> Result<(), DomainError> {
    tracing::info!("Pack Update RAW -> HQ: {pack_dir:?} -> {root_dir:?} (sync from {sync_dir:?})");

    if !pack_dir.is_dir() {
        tracing::info!("Pack dir is not vaild dir.");
        return Err(DomainError::Archive(anyhow::anyhow!(
            "Pack dir is not a valid directory"
        )));
    }
    if root_dir.exists() {
        return Err(DomainError::Archive(anyhow::anyhow!(
            "Directory {} already exists",
            root_dir.display()
        )));
    }
    if !sync_dir.is_dir() {
        tracing::info!("Syncing dir is not vaild dir.");
        return Err(DomainError::Archive(anyhow::anyhow!(
            "Sync dir is not a valid directory"
        )));
    }
    fs::create_dir_all(root_dir).await?;
    let cache_dir = root_dir.join("CacheDir");

    // Step 1: Unzip packs
    tracing::info!("Unzipping packs from {pack_dir:?} to {root_dir:?}");
    unzip_numeric_to_bms_folder(pack_dir, &cache_dir, root_dir).await?;

    // Step 2: Sync dir names from sync_dir
    tracing::info!("Syncing dir name from {sync_dir:?} to {root_dir:?}");
    copy_numbered_workdir_names(sync_dir, root_dir).await?;

    // Step 3: Convert WAV -> FLAC
    tracing::info!("Parsing Audio... Phase 1: WAV -> FLAC");
    let flac_preset = AUDIO_PRESET_FLAC.clone();
    let flac_ffmpeg_preset = AUDIO_PRESET_FLAC_FFMPEG.clone();
    bms_folder_transfer_audio(
        root_dir,
        &["wav"],
        &[flac_preset, flac_ffmpeg_preset],
        &TransferOptions {
            origin_removal: OriginRemoval::Always,
            remove_existing_target_file: true,
            stop_on_error: false,
        },
    )
    .await?;

    // Step 4: Remove unnecessary media files
    tracing::info!("Removing Unneed Files");
    remove_unneed_media_files(root_dir, get_remove_media_rule_oraja()).await?;

    // Step 5: Soft sync from root_dir into sync_dir
    tracing::info!("Syncing dir files from {root_dir:?} to {sync_dir:?}");
    sync_folder(root_dir, sync_dir, &SYNC_PRESET_FOR_APPEND, 8).await?;

    // Step 6: Remove empty folders
    tracing::info!("Removing empty folder in {root_dir:?}");
    remove_empty_dirs(root_dir).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_pack_raw_to_hq_does_not_panic() {
        let root = TempDir::new().unwrap();
        let work = root.path().join("TestSong");
        fs::create_dir_all(&work).await.unwrap();
        fs::write(work.join("test.wav"), "fake-wav-data")
            .await
            .unwrap();
        let _result = pack_raw_to_hq(root.path()).await;
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
        let _result = pack_hq_to_lq(root.path()).await;
    }
}
