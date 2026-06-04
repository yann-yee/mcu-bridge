/// Session 状态机管理 — 调试会话生命周期。
///
/// 设计文档 §4.2 定义了 HALTED / RUNNING / RECOVERING 三态状态机。
/// 初始状态为 HALTED，不自动 continue——让用户/Agent 先设断点和 watch。
use log::info;

use crate::config::ChipConfig;
use crate::probe::DebugProbe;

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
    /// 调试探针后端
    pub backend: Box<dyn DebugProbe>,
}

impl Session {
    /// 连接探针并创建会话（初始状态 Halted）。
    ///
    /// 调用方负责创建并传入 `backend`，可注入 mock 便于测试。
    pub fn attach(chip: &ChipConfig, backend: Box<dyn DebugProbe>) -> anyhow::Result<Self> {
        let mut backend = backend;
        backend.attach(chip)?;
        let core_count = backend.core_count();
        info!("session attached to {} ({} core(s))", chip.name, core_count);
        Ok(Self {
            state: SessionState::Halted,
            chip_name: chip.name.clone(),
            core_count,
            pc: None,
            bp_count: 0,
            watch_count: 0,
            backend,
        })
    }

    /// 安全断开探针连接。
    pub fn detach(&mut self) -> anyhow::Result<()> {
        info!("detaching session from {}", self.chip_name);
        self.backend.detach()
    }

    /// 创建一个初始状态为 Halted 的会话（无后端连接）。
    ///
    /// ⚠ 此方法仅用于无需真实探针连接的场景（如测试）。
    /// 常规使用请用 [`Session::attach`]。
    #[deprecated(since = "0.1.0", note = "use Session::attach() instead")]
    pub fn new(chip_name: String) -> Self {
        info!("session created for chip: {}", chip_name);
        Self {
            state: SessionState::Halted,
            chip_name,
            core_count: 0,
            pc: None,
            bp_count: 0,
            watch_count: 0,
            backend: Box::new(crate::probe::probe_rs::ProbeRsBackend::new()),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self {
            state: SessionState::Halted,
            chip_name: "unknown".into(),
            core_count: 0,
            pc: None,
            bp_count: 0,
            watch_count: 0,
            backend: Box::new(crate::probe::probe_rs::ProbeRsBackend::new()),
        }
    }
}
