use std::path::PathBuf;

use clap::Subcommand;

/// 赛事操作命令
#[derive(Subcommand)]
pub enum EventCmd {
    /// 检查编号文件夹数量是否匹配
    CheckNumFolder {
        /// 目标路径
        path: PathBuf,
        /// 期望数量
        count: i32,
    },
    /// 创建指定数量的编号文件夹
    CreateNumFolders {
        /// 目标路径
        path: PathBuf,
        /// 要创建的文件夹数量
        count: i32,
    },
    /// 生成工作信息表（xlsx）
    GenerateWorkInfoTable {
        /// 目标路径
        path: PathBuf,
    },
}
