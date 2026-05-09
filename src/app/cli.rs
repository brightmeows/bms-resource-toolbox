//! CLI interface using clap derive.
//!
//! Provides subcommands for each option function in the toolbox.

use std::path::PathBuf;

pub mod dispatch;

use clap::{Parser, Subcommand, ValueEnum};

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
    /// BMS 根目录操作
    #[command(subcommand)]
    Folder(FolderCommands),
    /// BMS 大包操作
    #[command(subcommand)]
    Pack(PackCommands),
    /// 音视频转换
    #[command(subcommand)]
    Media(MediaCommands),
    /// 源文件解压/编号
    #[command(subcommand)]
    Source(SourceCommands),
    /// 活动管理
    #[command(subcommand)]
    Event(EventCommands),
}

// ── folder ──────────────────────────────────────────────

/// BMS 根目录操作
#[derive(Subcommand)]
pub enum FolderCommands {
    /// 按 BMS 信息设置/追加文件夹名
    SetName {
        /// Naming mode (set-all, set-title, set-artist, append-all, append-title, append-artist)
        #[arg(short, long, default_value = "set-all")]
        mode: SetNameMode,
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
    /// 撤销重命名
    Undo {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
    /// 克隆带编号的文件夹名
    CopyNumbered {
        /// Source directory path
        #[arg(short, long)]
        from: PathBuf,
        /// Destination directory path
        #[arg(short, long)]
        to: PathBuf,
    },
    /// 扫描相似文件夹名
    ScanSimilar {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
    /// 清理空尺寸媒体文件和临时文件
    RemoveZeroMedia {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
}

/// 文件夹命名模式
#[derive(ValueEnum, Clone, Debug)]
pub enum SetNameMode {
    /// 设置为「标题 \[艺术家\]」
    SetAll,
    /// 仅设置为标题
    SetTitle,
    /// 仅设置为艺术家
    SetArtist,
    /// 追加「标题 \[艺术家\]」（仅纯数字目录）
    AppendAll,
    /// 追加标题
    AppendTitle,
    /// 追加 \[艺术家\]
    AppendArtist,
}

// ── pack ────────────────────────────────────────────────

/// BMS 大包操作
#[derive(Subcommand)]
pub enum PackCommands {
    /// 按首字符（A-Z、平假名、片假名、汉字等）分类文件夹
    Split {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
    /// 撤销首字符分类
    UndoSplit {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
    /// 跨目录移动/合并作品
    MoveIn {
        /// Source directory path
        #[arg(short, long)]
        from: PathBuf,
        /// Destination directory path
        #[arg(short, long)]
        to: PathBuf,
    },
    /// 移出一层目录
    MoveOut {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
    /// 合并文件名相似的子文件夹
    MergeSameName {
        /// Source directory path
        #[arg(short, long)]
        from: PathBuf,
        /// Destination directory path
        #[arg(short, long)]
        to: PathBuf,
    },
    /// 将文件名相似的子文件夹合并到各平级目录
    MergeToSiblings {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
    /// 合并被拆分的文件夹
    MergeSplit {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
    /// 初始打包：原包→HQ 版
    RawHqSetup {
        /// Pack archive file path
        #[arg(short, long)]
        pack: PathBuf,
        /// Root directory path
        #[arg(short, long)]
        root: PathBuf,
    },
    /// 更新打包：原包→HQ 版（差分包）
    RawHqUpdate {
        /// Pack archive file path
        #[arg(short, long)]
        pack: PathBuf,
        /// Root directory path
        #[arg(short, long)]
        root: PathBuf,
        /// Sync directory path
        #[arg(short, long)]
        sync: PathBuf,
    },
    /// 已缓存原包→HQ 版大包
    RawToHq {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
    /// HQ 版→LR2 兼容 LQ 版大包
    HqToLq {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
}

// ── media ───────────────────────────────────────────────

/// 音视频格式转换
#[derive(Subcommand)]
pub enum MediaCommands {
    /// 音频格式转换（WAV/FLAC/OGG 互转）
    Audio {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
        /// Audio conversion mode (0=WAV→FLAC, 1=FLAC→OGG, 2=WAV→OGG, 3=FLAC→WAV)
        #[arg(short, long, default_value_t = 0)]
        mode: usize,
    },
    /// 视频格式转换（MP4→AVI/WMV/MPEG）
    Video {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
        /// Video format (0=AVI 512x512, 1=WMV2 512x512, 2=MPEG1 512x512)
        #[arg(short, long, default_value_t = 0)]
        format: usize,
    },
}

// ── source ──────────────────────────────────────────────

/// 源文件解压/编号
#[derive(Subcommand)]
pub enum SourceCommands {
    /// 解压编号文件至编号目录
    UnzipNumeric {
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
    /// 按原名解压至对应目录
    UnzipNamed {
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
    /// 为文件添加数字编号前缀
    SetNumber {
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
}

// ── event ───────────────────────────────────────────────

/// BMS 活动管理
#[derive(Subcommand)]
pub enum EventCommands {
    /// 跳转至 BMS 活动作品信息页
    Jump {
        /// BMS event (20=BOFTT, 21=BOF21, 103=LetsBMSEdit3)
        #[arg(short, long, default_value_t = BMSEvent::BOFTT as i32)]
        event: i32,
        /// Work ID(s) to open; if empty, opens event list
        #[arg(short, long)]
        work_id: Vec<i32>,
    },
    /// 检查编号对应文件夹是否存在
    CheckFolders {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
        /// Number of folders to check
        #[arg(short, long)]
        count: i32,
    },
    /// 创建编号空文件夹
    CreateFolders {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
        /// Number of folders to create
        #[arg(short, long)]
        count: i32,
    },
    /// 生成活动作品 xlsx 表格
    GenerateTable {
        /// Root directory path
        #[arg(short, long)]
        path: PathBuf,
    },
}

pub use dispatch::dispatch;
