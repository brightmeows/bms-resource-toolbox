//! Interactive folder commands.

use async_trait::async_trait;
use bms_res_tb_domain::error::DomainError;
use bms_res_tb_domain::folder::cleanup;
use bms_res_tb_domain::folder::rename;
use bms_res_tb_domain::folder::scan;

use crate::interactive::Session;
use crate::interactive::history;
use crate::interactive::output::print_msg;
use crate::interactive::path_validate::{expand_tilde, validate_path};
use crate::interactive::trait_def::InteractiveCommand;
use crate::interactive::types::{ParamDef, ParamValue, PathSemantic};

// ── Rename ─────────────────────────────────────────────

/// Rename folder by BMS info with mode selection.
///
/// Prompts mode first, then path — so the user knows what to fill in.
pub struct Rename;

#[async_trait]
impl InteractiveCommand for Rename {
    fn menu_name(&self) -> &'static str {
        "重命名文件夹（按 BMS 信息）"
    }

    fn params(&self) -> Vec<ParamDef> {
        Vec::new()
    }

    async fn execute(&self, _args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        // Step 1: Select mode
        let modes = [
            "设置为「标题 [艺术家]」",
            "仅设置为标题",
            "仅设置为艺术家",
            "追加「标题 [艺术家]」（仅纯数字目录）",
            "追加标题",
            "追加 [艺术家]",
        ];
        let Ok(mode) = inquire::Select::new("选择命名模式:", modes.to_vec()).prompt() else {
            print_msg!("已取消。");
            return Ok(());
        };

        // Step 2: Prompt for path
        let path = loop {
            let Some(path_str) = history::prompt_with_history("BMS 根目录:") else {
                print_msg!("已取消。");
                return Ok(());
            };
            let trimmed = path_str.trim().to_string();
            if trimmed.is_empty() {
                continue;
            }
            let expanded = expand_tilde(&trimmed);
            if let Err(msg) = validate_path(&expanded, PathSemantic::RootDir) {
                print_msg!("  ⚠ {msg}");
                continue;
            }
            break expanded;
        };

        // Step 3: Execute
        match mode {
            "设置为「标题 [艺术家]」" => rename::set_name_by_bms(&path).await,
            "仅设置为标题" => rename::set_title_by_bms(&path).await,
            "仅设置为艺术家" => rename::set_artist_by_bms(&path).await,
            "追加「标题 [艺术家]」（仅纯数字目录）" => {
                rename::append_name_by_bms(&path).await
            }
            "追加标题" => rename::append_title_by_bms(&path).await,
            "追加 [艺术家]" => rename::append_artist_name_by_bms(&path).await,
            _ => {
                print_msg!("已取消。");
                Ok(())
            }
        }
    }
}

// ── UndoSetName ─────────────────────────────────────────

/// Undo rename: remove " [artist]" suffix.
pub struct UndoSetName;

#[async_trait]
impl InteractiveCommand for UndoSetName {
    fn menu_name(&self) -> &'static str {
        "撤销重命名"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("BMS 根目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let path = args
            .into_iter()
            .next()
            .expect("UndoSetName: missing path")
            .into_path();
        rename::undo_set_name(&path).await
    }
}

// ── CopyNumbered ────────────────────────────────────────

/// Copy numbered folder names from source to destination.
pub struct CopyNumbered;

#[async_trait]
impl InteractiveCommand for CopyNumbered {
    fn menu_name(&self) -> &'static str {
        "克隆带编号的文件夹名"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::root_dir("源根目录（已有带编号目录名）"),
            ParamDef::root_dir("目标根目录（只有编号的目录）"),
        ]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let from = iter.next().expect("CopyNumbered: missing from").into_path();
        let to = iter.next().expect("CopyNumbered: missing to").into_path();
        cleanup::copy_numbered_workdir_names(&from, &to).await
    }
}

// ── ScanSimilar ─────────────────────────────────────────

/// Scan for similar folder names.
pub struct ScanSimilar;

#[async_trait]
impl InteractiveCommand for ScanSimilar {
    fn menu_name(&self) -> &'static str {
        "扫描相似文件夹名"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("BMS 根目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let path = args
            .into_iter()
            .next()
            .expect("ScanSimilar: missing path")
            .into_path();
        scan::scan_folder_similar_folders(&path, 0.7).await
    }
}

// ── RemoveZeroMedia ─────────────────────────────────────

/// Remove zero-sized media files and temp files.
pub struct RemoveZeroMedia;

#[async_trait]
impl InteractiveCommand for RemoveZeroMedia {
    fn menu_name(&self) -> &'static str {
        "清理空尺寸媒体文件和临时文件"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("BMS 根目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let path = args
            .into_iter()
            .next()
            .expect("RemoveZeroMedia: missing path")
            .into_path();
        cleanup::remove_zero_sized_media_files(&path, false).await
    }
}
