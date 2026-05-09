//! CLI command dispatch.

use super::{
    Commands, EventCommands, FolderCommands, MediaCommands, PackCommands, RemoveMediaPreset,
    SetNameMode, SourceCommands,
};
use crate::domain::error::DomainError;

/// Prompt user for confirmation. Skips if `yes` is true.
fn confirm_action(action: &str, target: &str, yes: bool) -> Result<(), DomainError> {
    if yes {
        return Ok(());
    }
    tracing::info!("下列操作需要确认：");
    tracing::info!("  {action}: {target}");
    if !dialoguer::Confirm::new()
        .with_prompt("继续？")
        .default(false)
        .interact()
        .map_err(std::io::Error::other)?
    {
        tracing::info!("已取消。");
        return Err(DomainError::Cancelled);
    }
    Ok(())
}

/// Dispatch a CLI command to the appropriate domain function.
///
/// # Errors
///
/// Returns [`DomainError`] if the underlying domain operation fails.
pub async fn dispatch(cmd: &Commands, yes: bool) -> Result<(), DomainError> {
    match cmd {
        Commands::Folder(cmd) => dispatch_folder(cmd).await,
        Commands::Pack(cmd) => dispatch_pack(cmd, yes).await,
        Commands::Media(cmd) => dispatch_media(cmd).await,
        Commands::Source(cmd) => dispatch_source(cmd).await,
        Commands::Event(cmd) => dispatch_event(cmd).await,
    }
}

// ── folder ──────────────────────────────────────────────

async fn dispatch_folder(cmd: &FolderCommands) -> Result<(), DomainError> {
    match cmd {
        FolderCommands::SetName { mode, path } => match mode {
            SetNameMode::SetAll => crate::domain::folder::rename::set_name_by_bms(path).await,
            SetNameMode::SetTitle => crate::domain::folder::rename::set_title_by_bms(path).await,
            SetNameMode::SetArtist => crate::domain::folder::rename::set_artist_by_bms(path).await,
            SetNameMode::AppendAll => crate::domain::folder::rename::append_name_by_bms(path).await,
            SetNameMode::AppendTitle => {
                crate::domain::folder::rename::append_title_by_bms(path).await
            }
            SetNameMode::AppendArtist => {
                crate::domain::folder::rename::append_artist_name_by_bms(path).await
            }
        },
        FolderCommands::Undo { path } => crate::domain::folder::rename::undo_set_name(path).await,
        FolderCommands::CopyNumbered { from, to } => {
            crate::domain::folder::cleanup::copy_numbered_workdir_names(from, to).await
        }
        FolderCommands::ScanSimilar { path } => {
            crate::domain::folder::scan::scan_folder_similar_folders(path, 0.7).await
        }
        FolderCommands::RemoveZeroMedia { path } => {
            crate::domain::folder::cleanup::remove_zero_sized_media_files(path, false).await
        }
    }
}

// ── pack ────────────────────────────────────────────────

async fn dispatch_pack(cmd: &PackCommands, yes: bool) -> Result<(), DomainError> {
    match cmd {
        PackCommands::Split { path } => {
            crate::domain::folder::pack::split_folders_with_first_char(path).await
        }
        PackCommands::UndoSplit { path } => {
            confirm_action("撤销首字符拆分", &path.display().to_string(), yes)?;
            crate::domain::folder::pack::undo_split_pack(path).await
        }
        PackCommands::MoveIn { from, to } => {
            crate::domain::folder::pack::move_works_in_pack(from, to).await
        }
        PackCommands::MoveOut { path } => crate::domain::folder::pack::move_out_works(path).await,
        PackCommands::MergeSameName { from, to } => {
            confirm_action(
                "合并同名文件夹",
                &format!("{} → {}", from.display(), to.display()),
                yes,
            )?;
            crate::domain::folder::pack::move_works_with_same_name(from, to).await
        }
        PackCommands::MergeToSiblings { path } => {
            confirm_action("合并至平级目录", &path.display().to_string(), yes)?;
            crate::domain::folder::pack::move_works_with_same_name_to_siblings(path).await
        }
        PackCommands::MergeSplit { path } => {
            crate::domain::folder::pack::merge_split_folders(path).await
        }
        PackCommands::RawHqSetup { pack, root } => {
            crate::domain::pack::generate::pack_setup_rawpack_to_hq(pack, root).await
        }
        PackCommands::RawHqUpdate { pack, root, sync } => {
            crate::domain::pack::generate::pack_update_rawpack_to_hq(pack, root, sync).await
        }
        PackCommands::RawToHq { path } => crate::domain::pack::generate::pack_raw_to_hq(path).await,
        PackCommands::HqToLq { path } => crate::domain::pack::generate::pack_hq_to_lq(path).await,
    }
}

// ── media ───────────────────────────────────────────────

async fn dispatch_media(cmd: &MediaCommands) -> Result<(), DomainError> {
    match cmd {
        MediaCommands::Audio { path, mode } => {
            crate::domain::transfer::transfer_audio(path, *mode).await
        }
        MediaCommands::Video { path, format } => {
            crate::domain::transfer::transfer_video(path, *format).await
        }
        MediaCommands::RemoveUnneed { path, preset } => {
            let rule = match preset {
                RemoveMediaPreset::WavFlac => {
                    crate::domain::folder::media::get_remove_media_rule_wav_flac()
                }
                RemoveMediaPreset::MpgWmv => {
                    crate::domain::folder::media::get_remove_media_rule_mpg_wmv()
                }
                RemoveMediaPreset::Oraja => {
                    crate::domain::folder::media::get_remove_media_rule_oraja()
                }
            };
            crate::domain::folder::media::remove_unneed_media_files(path, rule).await
        }
    }
}

// ── source ──────────────────────────────────────────────

async fn dispatch_source(cmd: &SourceCommands) -> Result<(), DomainError> {
    match cmd {
        SourceCommands::UnzipNumeric { pack, cache, root } => {
            crate::domain::pack::unzip_numeric::unzip_numeric_to_bms_folder(pack, cache, root).await
        }
        SourceCommands::UnzipNamed { pack, cache, root } => {
            crate::domain::pack::unzip_name::unzip_with_name_to_bms_folder(pack, cache, root).await
        }
        SourceCommands::SetNumber {
            path,
            file_idx,
            num,
        } => crate::domain::pack::unzip_numeric::set_file_num(path, *file_idx, *num).await,
    }
}

// ── event ───────────────────────────────────────────────

async fn dispatch_event(cmd: &EventCommands) -> Result<(), DomainError> {
    match cmd {
        EventCommands::Jump { event, work_id } => {
            crate::domain::event::jump::jump_to_work_info(*event, work_id);
            Ok(())
        }
        EventCommands::CheckFolders { path, count } => {
            crate::domain::event::folder::check_num_folder(path, *count);
            Ok(())
        }
        EventCommands::CreateFolders { path, count } => {
            crate::domain::event::folder::create_num_folders(path, *count).await
        }
        EventCommands::GenerateTable { path } => {
            crate::domain::event::folder::generate_work_info_table(path).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirm_action_yes_skips() {
        let result = confirm_action("test", "target", true);
        assert!(result.is_ok(), "yes=true should skip confirmation");
    }
}
