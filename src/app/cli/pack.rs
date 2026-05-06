use std::path::PathBuf;

use clap::Subcommand;

/// 大包操作命令
#[derive(Subcommand)]
pub enum PackCmd {
    /// 按首字符拆分文件夹到大包
    SplitFoldersWithFirstChar {
        /// 目标路径
        path: PathBuf,
    },
    /// 撤销大包拆分
    UndoSplitPack {
        /// 目标路径
        path: PathBuf,
    },
    /// 在大包之间移动作品
    MoveWorksInPack {
        /// 源路径
        from: PathBuf,
        /// 目标路径
        to: PathBuf,
    },
    /// 将作品移出大包
    MoveOutWorks {
        /// 目标路径
        path: PathBuf,
    },
    /// 移动同名作品到目标大包
    MoveWorksWithSameName {
        /// 源路径
        from: PathBuf,
        /// 目标路径
        to: PathBuf,
    },
    /// 将同名作品移动到同级大包
    MoveWorksWithSameNameToSiblings {
        /// 目标路径
        path: PathBuf,
    },
    /// 合并已拆分的文件夹
    MergeSplitFolders {
        /// 目标路径
        path: PathBuf,
    },
}
