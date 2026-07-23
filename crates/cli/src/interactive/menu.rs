//! Interactive main menu.
//!
//! Displays commands grouped by module with tens-boundary numbering
//! (matching the Python bms-resource-scripts behavior).
//!
//! Each module group starts at the next multiple of 10 + 1.
//! For example: group 1 = 1..N, group 2 = 11..N2, group 3 = 21..N3, etc.

use std::collections::HashMap;

use bms_res_tb_domain::error::DomainError;

use super::Session;
use super::cmd::COMMAND_GROUPS;
use super::input::run_interactive_command;
use super::output::print_msg;
use super::trait_def::InteractiveCommand;

/// Build a map from user-facing menu number to command,
/// and display the grouped menu.
fn build_menu() -> HashMap<usize, &'static dyn InteractiveCommand> {
    let mut map: HashMap<usize, &'static dyn InteractiveCommand> = HashMap::new();
    let mut number = 1_usize;

    for group in COMMAND_GROUPS {
        print_msg!("");
        print_msg!("【{}】", group.name);

        for cmd in group.commands {
            map.insert(number, *cmd);
            print_msg!(" - {}: {}", number, cmd.menu_name());
            number += 1;
        }

        // Jump to next tens boundary (e.g., 4 → 11, 16 → 21)
        number = ((number - 1) / 10 + 1) * 10 + 1;
    }

    map
}

/// Run the interactive main menu loop.
///
/// # Errors
///
/// Returns an error if a terminal operation fails irrecoverably.
pub async fn run_main_menu(yes: bool) -> Result<(), DomainError> {
    let session = Session { yes };

    // Rebuild menu each iteration (static data, same result each time)
    loop {
        let cmd_map = build_menu();

        print_msg!("");
        let input = inquire::Text::new("输入要启用的功能的下标")
            .with_help_message("输入编号，按 Enter 确认")
            .prompt();

        let input = match input {
            Ok(s) => s.trim().to_string(),
            Err(inquire::InquireError::OperationCanceled) => {
                print_msg!("再见！");
                break;
            }
            Err(_) => continue,
        };

        if input.is_empty() {
            continue;
        }

        let Ok(num) = input.parse::<usize>() else {
            print_msg!("请重新输入");
            continue;
        };

        let Some(cmd) = cmd_map.get(&num).copied() else {
            print_msg!("请重新输入");
            continue;
        };

        if let Err(e) = run_interactive_command(cmd, session).await {
            if let DomainError::Cancelled = e {
                print_msg!("已取消。");
            } else {
                print_msg!("\n  ⚠ 操作遇到错误: {e}");
            }
        }
    }

    Ok(())
}
