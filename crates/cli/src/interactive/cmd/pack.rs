//! Interactive pack commands.

use async_trait::async_trait;
use bms_res_tb_domain::error::DomainError;
use bms_res_tb_domain::folder::pack;
use bms_res_tb_domain::pack::generate;

use crate::interactive::input::confirm_or_skip;
use crate::interactive::trait_def::InteractiveCommand;
use crate::interactive::types::{ParamDef, ParamValue};
use crate::interactive::Session;

// ── Split ───────────────────────────────────────────────

pub struct Split;

#[async_trait]
impl InteractiveCommand for Split {
    fn menu_name(&self) -> &'static str {
        "按首字符分类文件夹"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("大包目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("Split: missing path").into_path();
        pack::split_folders_with_first_char(&path).await
    }
}

// ── UndoSplit ───────────────────────────────────────────

pub struct UndoSplit;

#[async_trait]
impl InteractiveCommand for UndoSplit {
    fn menu_name(&self) -> &'static str {
        "撤销首字符分类"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("大包目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("UndoSplit: missing path").into_path();
        if !session.yes
            && !confirm_or_skip("撤销首字符拆分", &path.display().to_string())?
        {
            return Ok(());
        }
        pack::undo_split_pack(&path).await
    }
}

// ── MoveIn ──────────────────────────────────────────────

pub struct MoveIn;

#[async_trait]
impl InteractiveCommand for MoveIn {
    fn menu_name(&self) -> &'static str {
        "跨目录移动/合并作品"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::root_dir("源大包目录"),
            ParamDef::root_dir("目标大包目录"),
        ]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let from = iter.next().expect("MoveIn: missing from").into_path();
        let to = iter.next().expect("MoveIn: missing to").into_path();
        pack::move_works_in_pack(&from, &to).await
    }
}

// ── MoveOut ─────────────────────────────────────────────

pub struct MoveOut;

#[async_trait]
impl InteractiveCommand for MoveOut {
    fn menu_name(&self) -> &'static str {
        "移出一层目录（自动合并）"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("父目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("MoveOut: missing path").into_path();
        pack::move_out_works(&path).await
    }
}

// ── MergeSameName ───────────────────────────────────────

pub struct MergeSameName;

#[async_trait]
impl InteractiveCommand for MergeSameName {
    fn menu_name(&self) -> &'static str {
        "合并文件名相似的子文件夹"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::root_dir("源大包目录"),
            ParamDef::root_dir("目标大包目录"),
        ]
    }

    async fn execute(&self, args: Vec<ParamValue>, session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let from = iter.next().expect("MergeSameName: missing from").into_path();
        let to = iter.next().expect("MergeSameName: missing to").into_path();
        if !session.yes
            && !confirm_or_skip(
                "合并同名文件夹",
                &format!("{} → {}", from.display(), to.display()),
            )?
        {
            return Ok(());
        }
        pack::move_works_with_same_name(&from, &to).await
    }
}

// ── MergeToSiblings ─────────────────────────────────────

pub struct MergeToSiblings;

#[async_trait]
impl InteractiveCommand for MergeToSiblings {
    fn menu_name(&self) -> &'static str {
        "将相似子文件夹合并到平级目录"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("源大包目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("MergeToSiblings: missing path").into_path();
        if !session.yes
            && !confirm_or_skip("合并至平级目录", &path.display().to_string())?
        {
            return Ok(());
        }
        pack::move_works_with_same_name_to_siblings(&path).await
    }
}

// ── MergeSplit ──────────────────────────────────────────

pub struct MergeSplit;

#[async_trait]
impl InteractiveCommand for MergeSplit {
    fn menu_name(&self) -> &'static str {
        "合并被拆分的文件夹"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("大包目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("MergeSplit: missing path").into_path();
        pack::merge_split_folders(&path).await
    }
}

// ── RawHqSetup ──────────────────────────────────────────

pub struct RawHqSetup;

#[async_trait]
impl InteractiveCommand for RawHqSetup {
    fn menu_name(&self) -> &'static str {
        "初始打包：原包 → HQ 版"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::root_dir("原文件目录（pack dir）"),
            ParamDef::new_dir("输出根目录（不应已存在）"),
        ]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let pack = iter.next().expect("RawHqSetup: missing pack").into_path();
        let root = iter.next().expect("RawHqSetup: missing root").into_path();
        generate::pack_setup_rawpack_to_hq(&pack, &root).await
    }
}

// ── RawHqUpdate ─────────────────────────────────────────

pub struct RawHqUpdate;

#[async_trait]
impl InteractiveCommand for RawHqUpdate {
    fn menu_name(&self) -> &'static str {
        "更新打包：原包 → HQ 版（差分包）"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![
            ParamDef::root_dir("原文件目录（pack dir）"),
            ParamDef::new_dir("输出根目录（不应已存在）"),
            ParamDef::root_dir("已有 BMS 目录（用于同步）"),
        ]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let pack = iter.next().expect("RawHqUpdate: missing pack").into_path();
        let root = iter.next().expect("RawHqUpdate: missing root").into_path();
        let sync = iter.next().expect("RawHqUpdate: missing sync").into_path();
        generate::pack_update_rawpack_to_hq(&pack, &root, &sync).await
    }
}

// ── RawToHq ─────────────────────────────────────────────

pub struct RawToHq;

#[async_trait]
impl InteractiveCommand for RawToHq {
    fn menu_name(&self) -> &'static str {
        "已缓存原包 → HQ 版大包"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("BMS 根目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("RawToHq: missing path").into_path();
        generate::pack_raw_to_hq(&path).await
    }
}

// ── HqToLq ──────────────────────────────────────────────

pub struct HqToLq;

#[async_trait]
impl InteractiveCommand for HqToLq {
    fn menu_name(&self) -> &'static str {
        "HQ 版大包 → LR2 兼容 LQ 版"
    }

    fn params(&self) -> Vec<ParamDef> {
        vec![ParamDef::root_dir("BMS 根目录")]
    }

    async fn execute(&self, args: Vec<ParamValue>, _session: Session) -> Result<(), DomainError> {
        let mut iter = args.into_iter();
        let path = iter.next().expect("HqToLq: missing path").into_path();
        generate::pack_hq_to_lq(&path).await
    }
}
