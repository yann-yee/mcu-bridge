//! debug 子命令 — 启动调试会话。
//!
//! 设计文档 §4.2：双模式界面
//!   Human REPL — 交互式 `> ` 提示符，彩色输出
//!   Agent JSON-Lines — stdin→JSON，stdout→JSON，`--json` 模式
//!
//! 启动时先 `attach` 探针 → 进入 HALTED 态 → 等待用户/Agent 命令。

use std::path::PathBuf;

/// debug 子命令参数
pub struct DebugArgs {
    pub elf: PathBuf,
    pub config: Option<PathBuf>,
    pub json: bool,
    pub no_flash: bool,
    pub verify: bool,
    pub backend: Option<String>,
    pub enable_flash_bp: bool,
    pub break_at: Vec<String>,
    pub watch_targets: Vec<String>,
    pub continue_: bool,
    pub halt_on_start: bool,
    pub sampling_interval: Option<u64>,
    pub serial_port: Option<String>,
}

/// 处理 debug 子命令
pub fn handle(args: &DebugArgs) -> anyhow::Result<()> {
    let _ = args;
    todo!("debug: attach probe, parse config, start REPL/JSON-Lines loop")
}
