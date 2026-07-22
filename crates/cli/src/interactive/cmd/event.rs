//! Interactive event commands.

use async_trait::async_trait;
use bms_res_tb_domain::error::DomainError;
use bms_res_tb_domain::event::folder;
use bms_res_tb_domain::event::jump::{BMSEvent, jump_to_work_info};

use crate::interactive::output::print_msg;
use crate::interactive::trait_def::InteractiveCommand;
use crate::interactive::types::{ParamDef, ParamValue, PathSemantic};
use crate::interactive::Session;

// ── EventJump ───────────────────────────────────────────

pub struct EventJump;

#[async_trait]
impl InteractiveCommand for EventJump {
    fn menu_name(&self) -> &'static str {
        "跳转至 BMS 活动作品信息页"
    }

    fn params(&self) -> Vec<ParamDef> {
        Vec::new()
    }

    async fn execute(&self, _args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        interactive_event_jump()
    }
}

/// Interactive event jump with event selection and work ID input (with range support).
fn interactive_event_jump() -> Result<(), DomainError> {
    let event_options = ["BOFTT (BOF Team Festival)", "BOF2021", "LetsBMSEdit3"];
    let event_map = [BMSEvent::BOFTT, BMSEvent::BOF21, BMSEvent::LetsBMSEdit3];

    let Ok(selection) = inquire::Select::new("选择 BMS 活动:", event_options.to_vec()).prompt() else {
        print_msg!("已取消。");
        return Ok(());
    };

    let idx = event_options.iter().position(|o| *o == selection).unwrap_or(0);
    #[expect(clippy::indexing_slicing, reason = "idx validated by position() above")]
    let event = event_map[idx];

    print_msg!("");
    print_msg!("  → 已选: {event:?}");
    print_msg!("  !: 输入 \"1\": 跳转到作品 ID 1（单个）");
    print_msg!("  !: 输入 \"2 5\": 跳转到作品 ID 2、3、4、5（范围）");
    print_msg!("  !: 输入 \"2 5 6\": 跳转到作品 ID 2、5、6（列表）");
    print_msg!("  !: 按 Enter（空输入）: 跳转到活动列表页");
    print_msg!("  !: 按 Ctrl+C 退出");
    print_msg!("");

    loop {
        let input = inquire::Text::new("输入作品 ID:")
            .with_help_message("支持单个 / 范围 (空格) / 列表 (逗号或空格)")
            .prompt();

        let input = match input {
            Ok(s) => s.trim().to_string(),
            Err(inquire::InquireError::OperationCanceled) => {
                print_msg!("已取消。");
                return Ok(());
            }
            Err(_) => continue,
        };

        if input.is_empty() {
            jump_to_work_info(event, &[]);
            continue;
        }

        let normalized = input.replace(',', " ");
        let tokens: Vec<&str> = normalized.split_whitespace().filter(|t| !t.is_empty()).collect();

        if tokens.is_empty() {
            continue;
        }

        let nums: Vec<i32> = tokens.iter().filter_map(|t| t.parse::<i32>().ok()).collect();

        if nums.is_empty() {
            print_msg!("  ⚠ 请输入有效的数字。");
            continue;
        }

        let work_ids: Vec<i32> = if let [a, b] = nums.as_slice() {
            let start = (*a).min(*b);
            let end = (*a).max(*b);
            (start..=end).collect()
        } else {
            nums
        };

        print_msg!("  打开 {} 个作品页面...", work_ids.len());
        jump_to_work_info(event, &work_ids);
    }
}

// ── EventCheckFolders ───────────────────────────────────

pub struct EventCheckFolders;

#[async_trait]
impl InteractiveCommand for EventCheckFolders {
    fn menu_name(&self) -> &'static str {
        "检查编号对应文件夹是否存在"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::path(PathSemantic::Any, "活动根目录"),
            ParamDef::int(None, None, "检查数量"),
        ]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("EventCheckFolders: missing path arg").into_path();
        let count = iter.next().expect("EventCheckFolders: missing count arg").into_int();
        folder::check_num_folder(&path, count);
        Ok(())
    }
}

// ── EventCreateFolders ──────────────────────────────────

pub struct EventCreateFolders;

#[async_trait]
impl InteractiveCommand for EventCreateFolders {
    fn menu_name(&self) -> &'static str {
        "创建编号空文件夹"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::path(PathSemantic::Any, "活动根目录"),
            ParamDef::int(Some(1), None, "创建数量"),
        ]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("EventCreateFolders: missing path arg").into_path();
        let count = iter.next().expect("EventCreateFolders: missing count arg").into_int();
        folder::create_num_folders(&path, count).await
    }
}

// ── EventGenerateTable ──────────────────────────────────

pub struct EventGenerateTable;

#[async_trait]
impl InteractiveCommand for EventGenerateTable {
    fn menu_name(&self) -> &'static str {
        "生成活动作品 xlsx 表格"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::path(PathSemantic::Any, "活动根目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("EventGenerateTable: missing path arg").into_path();
        folder::generate_work_info_table(&path).await
    }
}
