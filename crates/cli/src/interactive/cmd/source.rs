//! Interactive source commands.

use async_trait::async_trait;
use std::path::Path;

use bms_res_tb_domain::error::DomainError;
use bms_res_tb_domain::pack::unzip_name;
use bms_res_tb_domain::pack::unzip_numeric;

use crate::interactive::trait_def::InteractiveCommand;
use crate::interactive::types::{ParamDef, ParamValue, PathSemantic};
use crate::interactive::Session;
use crate::interactive::output::print_msg;

// ── UnzipNumeric ────────────────────────────────────────

pub struct UnzipNumeric;

#[async_trait]
impl InteractiveCommand for UnzipNumeric {
    fn menu_name(&self) -> &'static str {
        "解压编号文件至编号目录"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::root_dir("原文件目录（pack dir）"),
            ParamDef::path(PathSemantic::Any, "缓存目录（cache dir）"),
            ParamDef::root_dir("目标 BMS 根目录"),
        ]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let pack = iter.next().expect("UnzipNumeric: missing pack").into_path();
        let cache = iter.next().expect("UnzipNumeric: missing cache").into_path();
        let root = iter.next().expect("UnzipNumeric: missing root").into_path();
        unzip_numeric::unzip_numeric_to_bms_folder(&pack, &cache, &root).await
    }
}

// ── UnzipNamed ──────────────────────────────────────────

pub struct UnzipNamed;

#[async_trait]
impl InteractiveCommand for UnzipNamed {
    fn menu_name(&self) -> &'static str {
        "按原名解压至对应目录"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::root_dir("原文件目录（pack dir）"),
            ParamDef::path(PathSemantic::Any, "缓存目录（cache dir）"),
            ParamDef::root_dir("目标 BMS 根目录"),
        ]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let pack = iter.next().expect("UnzipNamed: missing pack").into_path();
        let cache = iter.next().expect("UnzipNamed: missing cache").into_path();
        let root = iter.next().expect("UnzipNamed: missing root").into_path();
        unzip_name::unzip_with_name_to_bms_folder(&pack, &cache, &root).await
    }
}

// ── SetNumber ───────────────────────────────────────────

const ALLOWED_EXTS: &[&str] = &["zip", "7z", "rar", "mp4", "bms", "bme", "bml", "pms"];

pub struct SetNumber;

#[async_trait]
impl InteractiveCommand for SetNumber {
    fn menu_name(&self) -> &'static str {
        "为文件添加数字编号前缀"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("文件所在目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let dir = iter.next().expect("SetNumber: missing dir").into_path();

        // Interactive: Select file → enter number → loop
        loop {
            let Some((_file_name, _file_path)) = select_numberable_file(&dir) else {
                return Ok(());
            };

            // Prompt for number
            let Ok(num_input) = inquire::Text::new("输入编号:")
                .with_help_message("输入正整数")
                .prompt()
            else {
                print_msg!("已取消。");
                return Ok(());
            };

            let trimmed = num_input.trim().to_string();
            let Ok(num) = trimmed.parse::<i32>() else {
                print_msg!("  ⚠ 请输入有效的数字。");
                continue;
            };

            unzip_numeric::set_file_num(&dir, 0, num).await?;

            let cont = inquire::Confirm::new("继续处理其他文件?")
                .with_default(false)
                .prompt()
                .unwrap_or(false);

            if !cont {
                break;
            }
        }

        Ok(())
    }
}

/// Select a file from the directory that can be numbered.
fn select_numberable_file(dir: &Path) -> Option<(String, std::path::PathBuf)> {
    let files = get_numberable_files(dir);

    if files.is_empty() {
        print_msg!("该目录下没有可编号的文件。");
        return None;
    }

    let options: Vec<String> = files.iter().map(|(name, _)| name.clone()).collect();

    let selection = inquire::Select::new("选择要编号的文件:", options).prompt().ok()?;

    let idx = files.iter().position(|(name, _)| name == &selection)?;
    Some(files.get(idx)?.clone())
}

/// Get all files in the directory that are eligible for numbering.
fn get_numberable_files(dir: &Path) -> Vec<(String, std::path::PathBuf)> {
    let Ok(mut entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut result = Vec::new();

    while let Some(Ok(entry)) = entries.next() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();

        // Skip already numbered files
        if name.split_whitespace().next().is_some_and(|s| s.parse::<i32>().is_ok()) {
            continue;
        }

        // Skip empty files
        #[expect(clippy::disallowed_methods, reason = "sync context in interactive prompt")]
        if std::fs::metadata(&path).map_or(true, |m| m.len() == 0) {
            continue;
        }

        // Check extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !ALLOWED_EXTS.contains(&ext) {
            continue;
        }

        result.push((name, path));
    }

    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}
