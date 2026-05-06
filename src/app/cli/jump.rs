use crate::domain::service::jump::BMSEvent;

/// 活动跳转命令
#[derive(clap::Subcommand)]
pub enum JumpCmd {
    /// 按赛事与作品 ID 跳转到作品信息页
    WorkInfo {
        /// 赛事编号
        #[arg(short, long, default_value_t = BMSEvent::BOFTT as i32)]
        event: i32,
        /// 作品 ID 列表
        #[arg(short, long)]
        work_id: Vec<i32>,
    },
}
