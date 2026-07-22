//! Unified parameter input framework.
//!
//! Provides `prompt_params` which takes `&[ParamDef]` and prompts the user
//! for each parameter according to its type.

use std::path::PathBuf;

use bms_res_tb_domain::error::DomainError;

use super::history;
use super::path_validate::validate_path;
use super::trait_def::InteractiveCommand;
use super::types::{ParamDef, ParamType, ParamValue, PathSemantic};

/// Prompt the user for a yes/no confirmation.
///
/// Returns `true` if confirmed, `false` if declined.
/// Errors propagate as [`DomainError::Cancelled`] only on terminal failure.
pub fn confirm_or_skip(action: &str, target: &str) -> Result<bool, DomainError> {
    tracing::info!("下列操作需要确认：");
    tracing::info!("  {action}: {target}");
    let confirmed = dialoguer::Confirm::new()
        .with_prompt("继续？")
        .default(false)
        .interact()
        .map_err(std::io::Error::other)?;
    if !confirmed {
        tracing::info!("已取消。");
    }
    Ok(confirmed)
}

/// Prompt the user for all declared parameters.
///
/// Returns the resolved parameter values in order, or `None` if the user cancelled.
pub fn prompt_params(params: &[ParamDef]) -> Option<Vec<ParamValue>> {
    let mut args: Vec<ParamValue> = Vec::with_capacity(params.len());

    for (i, param) in params.iter().enumerate() {
        let prefix = format!("[{}/{}]", i + 1, params.len());
        let prompt_text = format!("{prefix} {}: ", param.description);

        let value = match param.param_type {
            ParamType::Path { semantic } => prompt_path(&prompt_text, semantic)?,
            ParamType::Int { min, max } => prompt_int(&prompt_text, min, max)?,
            ParamType::String => prompt_string(&prompt_text)?,
        };

        args.push(value);
    }

    Some(args)
}

/// Prompt for a path parameter with autocomplete + history + validation.
fn prompt_path(prompt: &str, semantic: PathSemantic) -> Option<ParamValue> {
    loop {
        let path_str = history::prompt_with_history(prompt)?;

        let trimmed = path_str.trim().to_string();
        if trimmed.is_empty() {
            tracing::info!("路径不能为空，请重新输入。");
            continue;
        }

        let path = PathBuf::from(&trimmed);

        if let Err(msg) = validate_path(&path, semantic) {
            tracing::info!("  ⚠ {msg}");
            continue;
        }

        return Some(ParamValue::Path(path));
    }
}

/// Prompt for an integer parameter.
fn prompt_int(prompt: &str, min: Option<i32>, max: Option<i32>) -> Option<ParamValue> {
    loop {
        let range_hint = match (min, max) {
            (Some(l), Some(u)) => format!(" ({l}–{u})"),
            (Some(l), None) => format!(" (≥{l})"),
            (None, Some(u)) => format!(" (≤{u})"),
            (None, None) => String::new(),
        };

        let input = inquire::Text::new(&format!("{prompt}{range_hint}"))
            .prompt()
            .ok()?;

        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            tracing::info!("请输入数字。");
            continue;
        }

        let Ok(num) = trimmed.parse::<i32>() else {
            tracing::info!("  ⚠ 请输入有效的整数。");
            continue;
        };

        if let Some(lower) = min
            && num < lower
        {
            tracing::info!("  ⚠ 数字不能小于 {lower}。");
            continue;
        }
        if let Some(upper) = max
            && num > upper
        {
            tracing::info!("  ⚠ 数字不能大于 {upper}。");
            continue;
        }

        return Some(ParamValue::Int(num));
    }
}

/// Prompt for a simple string parameter.
fn prompt_string(prompt: &str) -> Option<ParamValue> {
    let input = inquire::Text::new(prompt).prompt().ok()?;
    Some(ParamValue::String(input))
}

/// Execute a command with parameter prompting.
pub async fn run_interactive_command(
    cmd: &dyn InteractiveCommand,
    session: super::Session,
) -> Result<(), DomainError> {
    tracing::info!("\n── {} ──", cmd.menu_name());

    let params = cmd.params();
    let Some(args) = prompt_params(&params) else {
        tracing::info!("已取消。");
        return Ok(());
    };

    cmd.execute(args, session).await
}
