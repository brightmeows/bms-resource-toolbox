//! Application bootstrapping — wire ports to adapters, create services.

use crate::domain::service::event::EventService;
use crate::domain::service::folder_cleanup::FolderCleanupService;
use crate::domain::service::folder_media::FolderMediaService;
use crate::domain::service::folder_rename::FolderRenameService;
use crate::domain::service::folder_scan::FolderScanService;
use crate::domain::service::jump::JumpService;
use crate::domain::service::pack_generate::PackGenerateService;
use crate::domain::service::pack_merge::PackMergeService;
use crate::domain::service::pack_move::PackMoveService;
use crate::domain::service::pack_split::PackSplitService;
use crate::domain::service::transfer::TransferService;
use crate::domain::service::unzip::UnzipService;
use crate::infra::adapters::browser::WebBrowserAdapter;
use crate::infra::adapters::fs::TokioFsAdapter;
use crate::infra::adapters::output::ConsoleOutput;
use crate::infra::adapters::xlsx::RustXlsxWriterAdapter;

/// Application context — holds all service instances.
pub struct AppContext {
    /// 文件夹扫描服务
    pub folder_scan: FolderScanService,
    /// 文件夹重命名服务
    pub folder_rename: FolderRenameService,
    /// 文件夹清理服务
    pub folder_cleanup: FolderCleanupService,
    /// 文件夹媒体文件处理服务
    pub folder_media: FolderMediaService,
    /// 大包拆分服务
    pub pack_split: PackSplitService,
    /// 大包移动服务
    pub pack_move: PackMoveService,
    /// 大包合并服务
    pub pack_merge: PackMergeService,
    /// 大包生成服务
    pub pack_generate: PackGenerateService,
    /// 赛事服务
    pub event: EventService,
    /// 音视频转换服务
    pub transfer: TransferService,
    /// 解压服务
    pub unzip: UnzipService,
    /// 活动跳转服务
    pub jump: JumpService,
}

/// Bootstrap — create all adapters and services.
#[must_use]
pub fn bootstrap() -> AppContext {
    AppContext {
        folder_scan: FolderScanService::new(Box::new(TokioFsAdapter), Box::new(ConsoleOutput)),
        folder_rename: FolderRenameService::new(Box::new(TokioFsAdapter), Box::new(ConsoleOutput)),
        folder_cleanup: FolderCleanupService::new(
            Box::new(TokioFsAdapter),
            Box::new(ConsoleOutput),
        ),
        folder_media: FolderMediaService::new(Box::new(TokioFsAdapter), Box::new(ConsoleOutput)),
        pack_split: PackSplitService::new(Box::new(TokioFsAdapter), Box::new(ConsoleOutput)),
        pack_move: PackMoveService::new(Box::new(TokioFsAdapter), Box::new(ConsoleOutput)),
        pack_merge: PackMergeService::new(Box::new(TokioFsAdapter), Box::new(ConsoleOutput)),
        pack_generate: PackGenerateService::new(Box::new(TokioFsAdapter), Box::new(ConsoleOutput)),
        event: EventService::new(
            Box::new(TokioFsAdapter),
            Box::new(ConsoleOutput),
            Box::new(RustXlsxWriterAdapter),
        ),
        transfer: TransferService::new(Box::new(TokioFsAdapter), Box::new(ConsoleOutput)),
        unzip: UnzipService::new(Box::new(TokioFsAdapter), Box::new(ConsoleOutput)),
        jump: JumpService::new(Box::new(WebBrowserAdapter)),
    }
}
