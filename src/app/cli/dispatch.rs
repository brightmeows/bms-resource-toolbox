//! CLI command dispatch — routes sub-enum variants to `AppContext` services.

use std::future::Future;

use crate::app::bootstrap::AppContext;
use crate::domain::error::DomainError;
use crate::domain::service::jump::BMSEvent;
use crate::domain::service::transfer::{AudioMode, VideoFormat};

fn run_async(fut: impl Future<Output = Result<(), DomainError>>) {
    if let Err(e) = tokio::runtime::Handle::current().block_on(fut) {
        eprintln!("{e:#}");
    }
}

/// Dispatch a CLI command to the appropriate service.
pub fn dispatch(ctx: &AppContext, cmd: &super::Commands) {
    match cmd {
        super::Commands::Jump(sub) => dispatch_jump(ctx, sub),
        super::Commands::Folder(sub) => dispatch_folder(ctx, sub),
        super::Commands::Pack(sub) => dispatch_pack(ctx, sub),
        super::Commands::Event(sub) => dispatch_event(ctx, sub),
        super::Commands::Transfer(sub) => dispatch_transfer(ctx, sub),
        super::Commands::Unzip(sub) => dispatch_unzip(ctx, sub),
        super::Commands::Generate(sub) => dispatch_generate(ctx, sub),
    }
}

fn dispatch_jump(ctx: &AppContext, cmd: &super::jump::JumpCmd) {
    match cmd {
        super::jump::JumpCmd::WorkInfo { event, work_id } => {
            let event = BMSEvent::from_i32(*event);
            ctx.jump.jump_to_work_info(event, work_id);
        }
    }
}

fn dispatch_folder(ctx: &AppContext, cmd: &super::folder::FolderCmd) {
    match cmd {
        super::folder::FolderCmd::SetNameByBms { path } => {
            run_async(ctx.folder_rename.set_name_by_bms(path));
        }
        super::folder::FolderCmd::AppendNameByBms { path } => {
            run_async(ctx.folder_rename.append_name_by_bms(path));
        }
        super::folder::FolderCmd::AppendArtistNameByBms { path } => {
            run_async(ctx.folder_rename.append_artist_name_by_bms(path));
        }
        super::folder::FolderCmd::CopyNumberedWorkdirNames { from, to } => {
            run_async(ctx.folder_cleanup.copy_numbered_workdir_names(from, to));
        }
        super::folder::FolderCmd::ScanFolderSimilarFolders { path } => {
            run_async(ctx.folder_scan.scan_folder_similar_folders(path, 0.7));
        }
        super::folder::FolderCmd::UndoSetName { path } => {
            run_async(ctx.folder_rename.undo_set_name(path));
        }
        super::folder::FolderCmd::RemoveZeroSizedMediaFiles { path } => {
            run_async(
                ctx.folder_cleanup
                    .remove_zero_sized_media_files(path, false),
            );
        }
    }
}

fn dispatch_pack(ctx: &AppContext, cmd: &super::pack::PackCmd) {
    match cmd {
        super::pack::PackCmd::SplitFoldersWithFirstChar { path } => {
            run_async(ctx.pack_split.split_folders_with_first_char(path));
        }
        super::pack::PackCmd::UndoSplitPack { path } => {
            run_async(ctx.pack_split.undo_split_pack(path));
        }
        super::pack::PackCmd::MoveWorksInPack { from, to } => {
            run_async(ctx.pack_move.move_works_in_pack(from, to));
        }
        super::pack::PackCmd::MoveOutWorks { path } => {
            run_async(ctx.pack_move.move_out_works(path));
        }
        super::pack::PackCmd::MoveWorksWithSameName { from, to } => {
            run_async(ctx.pack_move.move_works_with_same_name(from, to));
        }
        super::pack::PackCmd::MoveWorksWithSameNameToSiblings { path } => {
            run_async(ctx.pack_move.move_works_with_same_name_to_siblings(path));
        }
        super::pack::PackCmd::MergeSplitFolders { path } => {
            run_async(ctx.pack_merge.merge_split_folders(path));
        }
    }
}

fn dispatch_event(ctx: &AppContext, cmd: &super::event::EventCmd) {
    match cmd {
        super::event::EventCmd::CheckNumFolder { path, count } => {
            tokio::runtime::Handle::current().block_on(ctx.event.check_num_folder(path, *count));
        }
        super::event::EventCmd::CreateNumFolders { path, count } => {
            run_async(ctx.event.create_num_folders(path, *count));
        }
        super::event::EventCmd::GenerateWorkInfoTable { path } => {
            run_async(ctx.event.generate_work_info_table(path));
        }
    }
}

fn dispatch_transfer(ctx: &AppContext, cmd: &super::transfer::TransferCmd) {
    match cmd {
        super::transfer::TransferCmd::Audio { path, mode } => {
            let mode = AudioMode::all()
                .get(*mode)
                .copied()
                .unwrap_or(AudioMode::WavToFlac);
            run_async(ctx.transfer.transfer_audio(path, mode));
        }
        super::transfer::TransferCmd::Video { path, format } => {
            let format = VideoFormat::all()
                .get(*format)
                .copied()
                .unwrap_or(VideoFormat::Avi);
            run_async(ctx.transfer.transfer_video(path, format));
        }
    }
}

fn dispatch_unzip(ctx: &AppContext, cmd: &super::unzip::UnzipCmd) {
    match cmd {
        super::unzip::UnzipCmd::NumericToBmsFolder { pack, cache, root } => {
            run_async(ctx.unzip.unzip_numeric_to_bms_folder(pack, cache, root));
        }
        super::unzip::UnzipCmd::WithNameToBmsFolder { pack, cache, root } => {
            run_async(ctx.unzip.unzip_with_name_to_bms_folder(pack, cache, root));
        }
        super::unzip::UnzipCmd::SetFileNum {
            path,
            file_idx,
            num,
        } => {
            run_async(ctx.unzip.set_file_num(path, *file_idx, *num));
        }
    }
}

fn dispatch_generate(ctx: &AppContext, cmd: &super::generate::GenerateCmd) {
    match cmd {
        super::generate::GenerateCmd::SetupRawpackToHq { pack, root } => {
            run_async(ctx.pack_generate.pack_setup_rawpack_to_hq(pack, root));
        }
        super::generate::GenerateCmd::UpdateRawpackToHq { pack, root, sync } => {
            run_async(
                ctx.pack_generate
                    .pack_update_rawpack_to_hq(pack, root, sync),
            );
        }
        super::generate::GenerateCmd::RawToHq { path } => {
            run_async(ctx.pack_generate.pack_raw_to_hq(path));
        }
        super::generate::GenerateCmd::HqToLq { path } => {
            run_async(ctx.pack_generate.pack_hq_to_lq(path));
        }
    }
}
