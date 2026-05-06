use std::path::PathBuf;

use clap::Subcommand;

/// 解压操作命令
#[derive(Subcommand)]
pub enum UnzipCmd {
    /// 将编号压缩包解压为 BMS 文件夹结构
    NumericToBmsFolder {
        /// 大包路径
        pack: PathBuf,
        /// 缓存路径
        cache: PathBuf,
        /// 根目录路径
        root: PathBuf,
    },
    /// 将含名称的压缩包解压为 BMS 文件夹结构
    WithNameToBmsFolder {
        /// 大包路径
        pack: PathBuf,
        /// 缓存路径
        cache: PathBuf,
        /// 根目录路径
        root: PathBuf,
    },
    /// 设置文件编号
    SetFileNum {
        /// 目标路径
        path: PathBuf,
        /// 文件索引
        #[arg(short, long, default_value_t = 0)]
        file_idx: usize,
        /// 要设置的文件编号
        #[arg(short, long, default_value_t = 0)]
        num: i32,
    },
}
