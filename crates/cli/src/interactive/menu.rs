//! Interactive main menu.
//!
//! Displays a flat list of all available interactive commands and loops
//! until the user chooses to exit.

use bms_res_tb_domain::error::DomainError;

use super::cmd::ALL_COMMANDS;
use super::input::run_interactive_command;
use super::output::print_msg;
use super::Session;

/// Run the interactive main menu loop.
///
/// # Errors
///
/// Returns an error if a terminal operation fails irrecoverably.
pub async fn run_main_menu(yes: bool) -> Result<(), DomainError> {
    let session = Session { yes };

    loop {
        let selection = show_menu();

        match selection {
            MenuAction::RunCommand(idx) => {
                if let Some(cmd) = ALL_COMMANDS.get(idx)
                    && let Err(e) = run_interactive_command(*cmd, session).await
                {
                    if let DomainError::Cancelled = e {
                        print_msg!("已取消。");
                    } else {
                        print_msg!("\n  ⚠ 操作遇到错误: {e}");
                    }
                }
            }
            MenuAction::Exit => {
                print_msg!("再见！");
                break;
            }
        }
    }

    Ok(())
}

/// The action selected by the user in the menu.
enum MenuAction {
    RunCommand(usize),
    Exit,
}

/// Display the menu and return the user's selection.
fn show_menu() -> MenuAction {
    loop {
        print_msg!("\n═══════════ BMS 资源工具箱 ═══════════");
        for (i, cmd) in ALL_COMMANDS.iter().enumerate() {
            print_msg!("  {:>2}: {}", i + 1, cmd.menu_name());
        }
        print_msg!("   0: 退出");
        print_msg!("─────────────────────────────────────────");

        let input = inquire::Text::new(&format!("输入编号 (0-{})", ALL_COMMANDS.len()))
            .with_help_message("按 Enter 确认")
            .prompt();

        let input = match input {
            Ok(s) => s.trim().to_string(),
            Err(inquire::InquireError::OperationCanceled) => return MenuAction::Exit,
            Err(_) => continue,
        };

        if input.is_empty() {
            continue;
        }

        if input == "0" {
            return MenuAction::Exit;
        }

        if let Ok(num) = input.parse::<usize>()
            && num >= 1
            && num <= ALL_COMMANDS.len()
        {
            return MenuAction::RunCommand(num - 1);
        }

        print_msg!("  ⚠ 无效输入，请输入 0 到 {} 之间的编号。", ALL_COMMANDS.len());
    }
}
