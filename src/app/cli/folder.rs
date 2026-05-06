use std::path::PathBuf;

use clap::Subcommand;

/// 文件夹操作命令
#[derive(Subcommand)]
pub enum FolderCmd {
    /// 根据 BMS 曲目名设置文件夹名
    SetNameByBms {
        /// 目标路径
        path: PathBuf,
    },
    /// 根据 BMS 曲目名追加文件夹名
    AppendNameByBms {
        /// 目标路径
        path: PathBuf,
    },
    /// 根据 BMS 作者名追加文件夹名
    AppendArtistNameByBms {
        /// 目标路径
        path: PathBuf,
    },
    /// 将编号工作目录名从一个路径复制到另一个
    CopyNumberedWorkdirNames {
        /// 源路径
        from: PathBuf,
        /// 目标路径
        to: PathBuf,
    },
    /// 扫描文件夹中相似的子文件夹
    ScanFolderSimilarFolders {
        /// 目标路径
        path: PathBuf,
    },
    /// 撤销 `SetNameByBms` 的重命名
    UndoSetName {
        /// 目标路径
        path: PathBuf,
    },
    /// 移除零大小的媒体文件
    RemoveZeroSizedMediaFiles {
        /// 目标路径
        path: PathBuf,
    },
}
