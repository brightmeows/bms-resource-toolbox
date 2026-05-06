//! CLI interface using clap derive.
//!
//! Provides subcommands for each option function in the toolbox.

use std::path::PathBuf;

pub mod dispatch;

use clap::{Parser, Subcommand};

use crate::domain::event::jump::BMSEvent;

/// BMS Resource Toolbox - CLI interface
#[derive(Parser)]
#[command(name = "bms-resource-toolbox")]
#[command(version, about = "BMS Resource Toolbox")]
pub struct Cli {
    /// Skip all confirmation prompts
    #[arg(short, long, global = true)]
    pub yes: bool,

    /// CLI subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

/// All available CLI subcommands.
#[derive(Subcommand)]
pub enum Commands {
    /// BMS活动：跳转至作品信息页
    JumpToWorkInfo {
        /// BMS event (20=BOFTT, 21=BOF21, 103=LetsBMSEdit3)
        #[arg(short, long, default_value_t = BMSEvent::BOFTT as i32)]
        event: i32,
        /// Work ID(s) to open; if empty, opens event list
        #[arg(short, long)]
        work_id: Vec<i32>,
    },

    /// BMS根目录：按照BMS设置文件夹名
    SetNameByBms {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS根目录：按照BMS追加文件夹名
    AppendNameByBms {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS根目录：按照BMS追加文件夹艺术家名
    AppendArtistNameByBms {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS根目录：克隆带编号的文件夹名
    CopyNumberedWorkdirNames {
        /// Source directory path
        #[arg(short, long)]
        from: PathBuf,
        /// Destination directory path
        #[arg(short, long)]
        to: PathBuf,
    },

    /// BMS根目录：扫描相似文件夹名
    ScanFolderSimilarFolders {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS根目录：撤销重命名
    UndoSetName {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS根目录：移除大小为0的媒体文件和临时文件
    RemoveZeroSizedMediaFiles {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS大包目录：按照首字符分成多个文件夹
    SplitFoldersWithFirstChar {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS大包目录：（撤销）按照首字符分成多个文件夹
    UndoSplitPack {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS大包目录：将目录A下的作品移动到目录B
    MoveWorksInPack {
        /// Source directory path
        #[arg(short, long)]
        from: PathBuf,
        /// Destination directory path
        #[arg(short, long)]
        to: PathBuf,
    },

    /// BMS大包父目录：移出一层目录
    MoveOutWorks {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS大包目录：合并文件名相似的子文件夹到目标
    MoveWorksWithSameName {
        /// Source directory path
        #[arg(short, long)]
        from: PathBuf,
        /// Destination directory path
        #[arg(short, long)]
        to: PathBuf,
    },

    /// BMS大包目录：将文件名相似的子文件夹合并到各平级目录
    MoveWorksWithSameNameToSiblings {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS大包目录：合并被拆分的文件夹
    MergeSplitFolders {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS活动目录：检查编号对应文件夹是否存在
    CheckNumFolder {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
        /// Number of folders to check
        #[arg(short, long)]
        count: i32,
    },

    /// BMS活动目录：创建只带有编号的空文件夹
    CreateNumFolders {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
        /// Number of folders to create
        #[arg(short, long)]
        count: i32,
    },

    /// BMS活动目录：生成活动作品xlsx表格
    GenerateWorkInfoTable {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS根目录：音频文件转换
    TransferAudio {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
        /// Audio conversion mode (0=WAV→FLAC, 1=FLAC→OGG, 2=WAV→OGG, 3=FLAC→WAV)
        #[arg(short, long, default_value_t = 0)]
        mode: usize,
    },

    /// BMS根目录：视频文件转换
    TransferVideo {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
        /// Video format (0=AVI 512x512, 1=WMV2 512x512, 2=MPEG1 512x512)
        #[arg(short, long, default_value_t = 0)]
        format: usize,
    },

    /// BMS原文件：解压编号文件至根目录
    UnzipNumericToBmsFolder {
        /// Pack archive file path
        #[arg(short, long)]
        pack: PathBuf,
        /// Cache directory path
        #[arg(short, long)]
        cache: PathBuf,
        /// Root directory path
        #[arg(short, long)]
        root: PathBuf,
    },

    /// BMS原文件：解压文件至根目录（按原名）
    UnzipWithNameToBmsFolder {
        /// Pack archive file path
        #[arg(short, long)]
        pack: PathBuf,
        /// Cache directory path
        #[arg(short, long)]
        cache: PathBuf,
        /// Root directory path
        #[arg(short, long)]
        root: PathBuf,
    },

    /// BMS原文件：赋予编号
    SetFileNum {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
        /// File index (0-based) to rename
        #[arg(short, long, default_value_t = 0)]
        file_idx: usize,
        /// Number prefix to assign
        #[arg(short, long, default_value_t = 0)]
        num: i32,
    },

    /// 大包生成脚本：原包 -> HQ版大包
    PackSetupRawpackToHq {
        /// Pack directory path
        #[arg(short, long)]
        pack: PathBuf,
        /// Root directory path
        #[arg(short, long)]
        root: PathBuf,
    },

    /// 大包更新脚本：原包 -> HQ版大包
    PackUpdateRawpackToHq {
        /// Pack directory path
        #[arg(short, long)]
        pack: PathBuf,
        /// Root directory path
        #[arg(short, long)]
        root: PathBuf,
        /// Sync directory path
        #[arg(short, long)]
        sync: PathBuf,
    },

    /// BMS大包脚本：原包 -> HQ版大包
    PackRawToHq {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },

    /// BMS大包脚本：HQ版大包 -> LQ版大包
    PackHqToLq {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
}

pub use dispatch::dispatch;
