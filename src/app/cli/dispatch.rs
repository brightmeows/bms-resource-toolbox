//! CLI command dispatch.

use std::future::Future;

use super::Commands;
use crate::domain::error::DomainError;
use crate::domain::event::jump::BMSEvent;
use crate::domain::transfer::{AudioMode, VideoFormat};

fn run_async(fut: impl Future<Output = Result<(), DomainError>>) {
    if let Err(e) = tokio::runtime::Handle::current().block_on(fut) {
        eprintln!("{e:#}");
    }
}

/// Dispatch a CLI command to the appropriate domain function.
#[allow(clippy::too_many_lines)]
pub fn dispatch(cmd: &Commands) {
    match cmd {
        Commands::JumpToWorkInfo { event, work_id } => {
            let event = BMSEvent::from_i32(*event);
            crate::domain::event::jump::jump_to_work_info(event, work_id);
        }
        Commands::SetNameByBms { path } => {
            run_async(crate::domain::folder::rename::set_name_by_bms(path));
        }
        Commands::AppendNameByBms { path } => {
            run_async(crate::domain::folder::rename::append_name_by_bms(path));
        }
        Commands::AppendArtistNameByBms { path } => {
            run_async(crate::domain::folder::rename::append_artist_name_by_bms(
                path,
            ));
        }
        Commands::CopyNumberedWorkdirNames { from, to } => {
            run_async(crate::domain::folder::cleanup::copy_numbered_workdir_names(
                from, to,
            ));
        }
        Commands::ScanFolderSimilarFolders { path } => {
            run_async(crate::domain::folder::scan::scan_folder_similar_folders(
                path, 0.7,
            ));
        }
        Commands::UndoSetName { path } => {
            run_async(crate::domain::folder::rename::undo_set_name(path));
        }
        Commands::RemoveZeroSizedMediaFiles { path } => {
            run_async(crate::domain::folder::cleanup::remove_zero_sized_media_files(path, false));
        }
        Commands::SplitFoldersWithFirstChar { path } => {
            run_async(crate::domain::folder::pack::split_folders_with_first_char(
                path,
            ));
        }
        Commands::UndoSplitPack { path } => {
            run_async(crate::domain::folder::pack::undo_split_pack(path));
        }
        Commands::MoveWorksInPack { from, to } => {
            run_async(crate::domain::folder::pack::move_works_in_pack(from, to));
        }
        Commands::MoveOutWorks { path } => {
            run_async(crate::domain::folder::pack::move_out_works(path));
        }
        Commands::MoveWorksWithSameName { from, to } => {
            run_async(crate::domain::folder::pack::move_works_with_same_name(
                from, to,
            ));
        }
        Commands::MoveWorksWithSameNameToSiblings { path } => {
            run_async(crate::domain::folder::pack::move_works_with_same_name_to_siblings(path));
        }
        Commands::MergeSplitFolders { path } => {
            run_async(crate::domain::folder::pack::merge_split_folders(path));
        }
        Commands::CheckNumFolder { path, count } => {
            crate::domain::event::folder::check_num_folder(path, *count);
        }
        Commands::CreateNumFolders { path, count } => {
            run_async(crate::domain::event::folder::create_num_folders(
                path, *count,
            ));
        }
        Commands::GenerateWorkInfoTable { path } => {
            run_async(crate::domain::event::folder::generate_work_info_table(path));
        }
        Commands::TransferAudio { path, mode } => {
            let mode = AudioMode::all()
                .get(*mode)
                .copied()
                .unwrap_or(AudioMode::WavToFlac);
            run_async(crate::domain::transfer::transfer_audio(path, mode));
        }
        Commands::TransferVideo { path, format } => {
            let format = VideoFormat::all()
                .get(*format)
                .copied()
                .unwrap_or(VideoFormat::Avi);
            run_async(crate::domain::transfer::transfer_video(path, format));
        }
        Commands::UnzipNumericToBmsFolder { pack, cache, root } => {
            run_async(
                crate::domain::pack::unzip_numeric::unzip_numeric_to_bms_folder(pack, cache, root),
            );
        }
        Commands::UnzipWithNameToBmsFolder { pack, cache, root } => {
            run_async(
                crate::domain::pack::unzip_name::unzip_with_name_to_bms_folder(pack, cache, root),
            );
        }
        Commands::SetFileNum {
            path,
            file_idx,
            num,
        } => {
            run_async(crate::domain::pack::unzip_numeric::set_file_num(
                path, *file_idx, *num,
            ));
        }
        Commands::PackSetupRawpackToHq { pack, root } => {
            run_async(crate::domain::pack::generate::pack_setup_rawpack_to_hq(
                pack, root,
            ));
        }
        Commands::PackUpdateRawpackToHq { pack, root, sync } => {
            run_async(crate::domain::pack::generate::pack_update_rawpack_to_hq(
                pack, root, sync,
            ));
        }
        Commands::PackRawToHq { path } => {
            run_async(crate::domain::pack::generate::pack_raw_to_hq(path));
        }
        Commands::PackHqToLq { path } => {
            run_async(crate::domain::pack::generate::pack_hq_to_lq(path));
        }
    }
}
