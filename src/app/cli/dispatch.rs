//! CLI command dispatch.

use super::Commands;
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
        Commands::JumpToWorkInfo { event, work_id } => {
            let event = BMSEvent::from_i32(*event);
            crate::domain::event::jump::jump_to_work_info(event, work_id);
            Ok(())
        }
        Commands::SetNameByBms { path } => {
            crate::domain::folder::rename::set_name_by_bms(path).await
        }
        Commands::AppendNameByBms { path } => {
            crate::domain::folder::rename::append_name_by_bms(path).await
        }
        Commands::AppendArtistNameByBms { path } => {
            crate::domain::folder::rename::append_artist_name_by_bms(path).await
        }
        Commands::CopyNumberedWorkdirNames { from, to } => {
            crate::domain::folder::cleanup::copy_numbered_workdir_names(from, to).await
        }
        Commands::ScanFolderSimilarFolders { path } => {
            crate::domain::folder::scan::scan_folder_similar_folders(path, 0.7).await
        }
        Commands::UndoSetName { path } => crate::domain::folder::rename::undo_set_name(path).await,
        Commands::RemoveZeroSizedMediaFiles { path } => {
            crate::domain::folder::cleanup::remove_zero_sized_media_files(path, false).await
        }
        Commands::SplitFoldersWithFirstChar { path } => {
            crate::domain::folder::pack::split_folders_with_first_char(path).await
        }
        Commands::UndoSplitPack { path } => {
            crate::domain::folder::pack::undo_split_pack(path).await
        }
        Commands::MoveWorksInPack { from, to } => {
            crate::domain::folder::pack::move_works_in_pack(from, to).await
        }
        Commands::MoveOutWorks { path } => crate::domain::folder::pack::move_out_works(path).await,
        Commands::MoveWorksWithSameName { from, to } => {
            crate::domain::folder::pack::move_works_with_same_name(from, to).await
        }
        Commands::MoveWorksWithSameNameToSiblings { path } => {
            crate::domain::folder::pack::move_works_with_same_name_to_siblings(path).await
        }
        Commands::MergeSplitFolders { path } => {
            crate::domain::folder::pack::merge_split_folders(path).await
        }
        Commands::CheckNumFolder { path, count } => {
            crate::domain::event::folder::check_num_folder(path, *count);
            Ok(())
        }
        Commands::CreateNumFolders { path, count } => {
            crate::domain::event::folder::create_num_folders(path, *count).await
        }
        Commands::GenerateWorkInfoTable { path } => {
            crate::domain::event::folder::generate_work_info_table(path).await
        }
        Commands::TransferAudio { path, mode } => {
            let mode = AudioMode::all()
                .get(*mode)
                .copied()
                .unwrap_or(AudioMode::WavToFlac);
            crate::domain::transfer::transfer_audio(path, mode).await
        }
        Commands::TransferVideo { path, format } => {
            let format = VideoFormat::all()
                .get(*format)
                .copied()
                .unwrap_or(VideoFormat::Avi);
            crate::domain::transfer::transfer_video(path, format).await
        }
        Commands::UnzipNumericToBmsFolder { pack, cache, root } => {
            crate::domain::pack::unzip_numeric::unzip_numeric_to_bms_folder(pack, cache, root).await
        }
        Commands::UnzipWithNameToBmsFolder { pack, cache, root } => {
            crate::domain::pack::unzip_name::unzip_with_name_to_bms_folder(pack, cache, root).await
        }
        Commands::SetFileNum {
            path,
            file_idx,
            num,
        } => crate::domain::pack::unzip_numeric::set_file_num(path, *file_idx, *num).await,
        Commands::PackSetupRawpackToHq { pack, root } => {
            crate::domain::pack::generate::pack_setup_rawpack_to_hq(pack, root).await
        }
        Commands::PackUpdateRawpackToHq { pack, root, sync } => {
            crate::domain::pack::generate::pack_update_rawpack_to_hq(pack, root, sync).await
        }
        Commands::PackRawToHq { path } => crate::domain::pack::generate::pack_raw_to_hq(path).await,
        Commands::PackHqToLq { path } => crate::domain::pack::generate::pack_hq_to_lq(path).await,
    }
}
