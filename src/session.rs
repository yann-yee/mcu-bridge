/// Session 状态机管理 — 调试会话生命周期。
///
/// 设计文档 §4.2 定义了 HALTED / RUNNING / RECOVERING 三态状态机。
/// 初始状态为 HALTED，不自动 continue——让用户/Agent 先设断点和 watch。
use log::info;

/// 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// 目标已暂停，可设断点/watch/读寄存器
    Halted,
    /// 目标全速运行中，定时采样线程活跃
    Running,
    /// 探针断连，正在尝试自动恢复
    Recovering,
}

/// 调试会话上下文
pub struct Session {
    /// 当前状态
    pub state: SessionState,
    /// 芯片名称
    pub chip_name: String,
    /// 探针检测到的核数
    pub core_count: usize,
    /// 当前 PC 值 (halted 时有效)
    pub pc: Option<u32>,
    /// 当前设置的断点数
    pub bp_count: usize,
    /// 当前设置的 watchpoint 数
    pub watch_count: usize,
}

impl Session {
    /// 创建一个初始状态为 Halted 的会话。
    pub fn new(chip_name: String) -> Self {
        info!("session created for chip: {}", chip_name);
        Self {
            state: SessionState::Halted,
            chip_name,
            core_count: 0,
            pc: None,
            bp_count: 0,
            watch_count: 0,
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new("unknown".into())
    }
}
