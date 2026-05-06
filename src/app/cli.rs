//! CLI interface using clap derive.

pub mod dispatch;

/// 赛事命令
pub mod event;
/// 文件夹命令
pub mod folder;
/// 生成命令
pub mod generate;
/// 跳转命令
pub mod jump;
/// 大包命令
pub mod pack;
/// 转换命令
pub mod transfer;
/// 解压命令
pub mod unzip;

use clap::{Parser, Subcommand};

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

/// Top-level CLI commands — each variant delegates to a domain-specific sub-enum.
#[derive(Subcommand)]
pub enum Commands {
    /// 活动跳转
    #[command(subcommand)]
    Jump(jump::JumpCmd),
    /// 文件夹操作
    #[command(subcommand)]
    Folder(folder::FolderCmd),
    /// 大包操作
    #[command(subcommand)]
    Pack(pack::PackCmd),
    /// 活动目录操作
    #[command(subcommand)]
    Event(event::EventCmd),
    /// 音视频转换
    #[command(subcommand)]
    Transfer(transfer::TransferCmd),
    /// 解压操作
    #[command(subcommand)]
    Unzip(unzip::UnzipCmd),
    /// 大包生成
    #[command(subcommand)]
    Generate(generate::GenerateCmd),
}

pub use dispatch::dispatch;
