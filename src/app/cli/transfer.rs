use std::path::PathBuf;

use clap::Subcommand;

/// 音视频转换命令
#[derive(Subcommand)]
pub enum TransferCmd {
    /// 音频转换
    Audio {
        /// 目标路径
        path: PathBuf,
        /// 转换模式
        #[arg(short, long, default_value_t = 0)]
        mode: usize,
    },
    /// 视频转换
    Video {
        /// 目标路径
        path: PathBuf,
        /// 输出格式
        #[arg(short, long, default_value_t = 0)]
        format: usize,
    },
}
