use std::path::PathBuf;

use clap::Subcommand;

/// 大包生成命令
#[derive(Subcommand)]
pub enum GenerateCmd {
    /// 初始化 Rawpack 转 HQ 的目录结构
    SetupRawpackToHq {
        /// 大包路径
        pack: PathBuf,
        /// 根目录路径
        root: PathBuf,
    },
    /// 更新 Rawpack 转 HQ 的同步配置
    UpdateRawpackToHq {
        /// 大包路径
        pack: PathBuf,
        /// 根目录路径
        root: PathBuf,
        /// 同步配置路径
        sync: PathBuf,
    },
    /// 将 Raw 格式转为 HQ 格式
    RawToHq {
        /// 目标路径
        path: PathBuf,
    },
    /// 将 HQ 格式转为 LQ 格式
    HqToLq {
        /// 目标路径
        path: PathBuf,
    },
}
