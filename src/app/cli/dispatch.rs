//! CLI command dispatch.

use super::{
    Commands, EventCommands, FolderCommands, MediaCommands, PackCommands, SetNameMode,
    SourceCommands,
};
use crate::domain::error::DomainError;
use crate::domain::event::jump::BMSEvent;
use crate::domain::transfer::{AudioMode, VideoFormat};

/// Dispatch a CLI command to the appropriate domain function.
///
/// # Errors
///
/// Returns [`DomainError`] if the underlying domain operation fails.
pub async fn dispatch(cmd: &Commands) -> Result<(), DomainError> {
    match cmd {
        Commands::Folder(cmd) => dispatch_folder(cmd).await,
        Commands::Pack(cmd) => dispatch_pack(cmd).await,
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

async fn dispatch_pack(cmd: &PackCommands) -> Result<(), DomainError> {
    match cmd {
        PackCommands::Split { path } => {
            crate::domain::folder::pack::split_folders_with_first_char(path).await
        }
        PackCommands::UndoSplit { path } => {
            crate::domain::folder::pack::undo_split_pack(path).await
        }
        PackCommands::MoveIn { from, to } => {
            crate::domain::folder::pack::move_works_in_pack(from, to).await
        }
        PackCommands::MoveOut { path } => crate::domain::folder::pack::move_out_works(path).await,
        PackCommands::MergeSameName { from, to } => {
            crate::domain::folder::pack::move_works_with_same_name(from, to).await
        }
        PackCommands::MergeToSiblings { path } => {
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
            let mode = AudioMode::all()
                .get(*mode)
                .copied()
                .unwrap_or(AudioMode::WavToFlac);
            crate::domain::transfer::transfer_audio(path, mode).await
        }
        MediaCommands::Video { path, format } => {
            let format = VideoFormat::all()
                .get(*format)
                .copied()
                .unwrap_or(VideoFormat::Avi);
            crate::domain::transfer::transfer_video(path, format).await
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
            let event = BMSEvent::from_i32(*event);
            crate::domain::event::jump::jump_to_work_info(event, work_id);
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
