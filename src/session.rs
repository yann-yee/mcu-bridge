/// Session 状态机管理 — 调试会话生命周期。
///
/// 设计文档 §4.2 定义了 HALTED / RUNNING / RECOVERING 三态状态机。
/// 初始状态为 HALTED，不自动 continue——让用户/Agent 先设断点和 watch。
use std::sync::{Arc, Mutex};

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
    #[allow(dead_code)]
    Recovering,
}

/// 调试会话上下文
pub struct Session {
    /// 当前状态
    pub state: SessionState,
    /// 芯片名称
    pub chip_name: String,
    /// Active backend identifier.
    pub backend_name: String,
    /// 探针检测到的核数
    pub core_count: usize,
    /// 当前 PC 值 (halted 时有效)
    pub pc: Option<u32>,
    /// 当前设置的断点数
    pub bp_count: usize,
    /// 当前设置的 watchpoint 数
    pub watch_count: usize,
    /// 调试探针后端（Arc<Mutex> 支持多线程共享）
    pub backend: Arc<Mutex<Box<dyn DebugProbe>>>,
}

impl Session {
    /// 连接探针并创建会话（初始状态 Halted）。
    ///
    /// 调用方负责创建并传入 `backend`，可注入 mock 便于测试。
    pub fn attach(chip: &ChipConfig, backend: Box<dyn DebugProbe>) -> anyhow::Result<Self> {
        let mut backend = backend;
        let backend_name = backend.backend_name().to_string();
        if let Err(error) = backend.attach(chip) {
            return Err(cleanup_after_attach_error(
                backend.as_mut(),
                error,
                "failed to attach debug backend",
            ));
        }

        if !backend.is_connected() {
            if let Err(error) = backend.try_recover() {
                return Err(cleanup_after_attach_error(
                    backend.as_mut(),
                    error,
                    "failed to recover debug backend after attach",
                ));
            }
            if !backend.is_connected() {
                let error = anyhow::anyhow!("debug backend remained disconnected after recovery");
                return Err(cleanup_after_attach_error(
                    backend.as_mut(),
                    error,
                    "failed to establish a connected debug session",
                ));
            }
        }

        if let Err(error) = backend.halt(None) {
            return Err(cleanup_after_attach_error(
                backend.as_mut(),
                error,
                "failed to halt target after attach",
            ));
        }

        let core_count = backend.core_count();
        info!("session attached to {} ({} core(s))", chip.name, core_count);
        Ok(Self {
            state: SessionState::Halted,
            chip_name: chip.name.clone(),
            backend_name,
            core_count,
            pc: None,
            bp_count: 0,
            watch_count: 0,
            backend: Arc::new(Mutex::new(backend)),
        })
    }

    /// 返回共享后端引用（供采样线程持有）。
    pub fn shared_backend(&self) -> Arc<Mutex<Box<dyn DebugProbe>>> {
        self.backend.clone()
    }

    /// 安全断开探针连接。
    pub fn detach(&mut self) -> anyhow::Result<()> {
        info!("detaching session from {}", self.chip_name);
        self.backend.lock().expect("backend lock").detach()
    }

    /// 创建一个初始状态为 Halted 的会话（无后端连接）。
    ///
    /// ⚠ 此方法仅用于无需真实探针连接的场景（如测试）。
    /// 常规使用请用 [`Session::attach`]。
    #[deprecated(since = "0.1.0", note = "use Session::attach() instead")]
    #[allow(dead_code)]
    pub fn new(chip_name: String) -> Self {
        info!("session created for chip: {}", chip_name);
        Self {
            state: SessionState::Halted,
            chip_name,
            backend_name: "probe-rs".into(),
            core_count: 0,
            pc: None,
            bp_count: 0,
            watch_count: 0,
            backend: Arc::new(Mutex::new(Box::new(
                crate::probe::probe_rs::ProbeRsBackend::new(),
            ))),
        }
    }
}

fn cleanup_after_attach_error(
    backend: &mut dyn DebugProbe,
    error: anyhow::Error,
    context: &'static str,
) -> anyhow::Error {
    let error = error.context(context);
    if let Err(cleanup_error) = backend.detach() {
        log::warn!("failed to detach debug backend after attach failure: {cleanup_error:#}");
    }
    error
}

impl Default for Session {
    fn default() -> Self {
        Self {
            state: SessionState::Halted,
            chip_name: "unknown".into(),
            backend_name: "probe-rs".into(),
            core_count: 0,
            pc: None,
            bp_count: 0,
            watch_count: 0,
            backend: Arc::new(Mutex::new(Box::new(
                crate::probe::probe_rs::ProbeRsBackend::new(),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use crate::config::{ChipConfig, FlashOpts};
    use crate::probe::{BpId, DebugProbe, WatchKind, WpId};

    use super::{Session, SessionState};

    #[derive(Default)]
    struct MockState {
        calls: Vec<&'static str>,
        connected: bool,
        running: bool,
        attach_error: bool,
        halt_error: bool,
        recover_connected: bool,
    }

    struct MockProbe {
        state: Arc<Mutex<MockState>>,
    }

    impl DebugProbe for MockProbe {
        fn backend_name(&self) -> &'static str {
            "session-mock"
        }

        fn attach(&mut self, _chip: &ChipConfig) -> anyhow::Result<()> {
            let mut state = self.state.lock().unwrap();
            state.calls.push("attach");
            if state.attach_error {
                anyhow::bail!("mock attach failure");
            }
            state.connected = state.recover_connected;
            state.running = true;
            Ok(())
        }

        fn detach(&mut self) -> anyhow::Result<()> {
            let mut state = self.state.lock().unwrap();
            state.calls.push("detach");
            state.connected = false;
            Ok(())
        }

        fn is_connected(&self) -> bool {
            let mut state = self.state.lock().unwrap();
            state.calls.push("is_connected");
            state.connected
        }

        fn try_recover(&mut self) -> anyhow::Result<()> {
            let mut state = self.state.lock().unwrap();
            state.calls.push("try_recover");
            state.connected = true;
            Ok(())
        }

        fn flash(&mut self, _elf: &Path, _opts: &FlashOpts) -> anyhow::Result<()> {
            Ok(())
        }

        fn halt(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
            let mut state = self.state.lock().unwrap();
            state.calls.push("halt");
            if state.halt_error {
                anyhow::bail!("mock halt failure");
            }
            state.running = false;
            Ok(())
        }

        fn resume(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
            let mut state = self.state.lock().unwrap();
            state.calls.push("resume");
            state.running = true;
            Ok(())
        }

        fn step(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
            Ok(())
        }

        fn core_count(&self) -> usize {
            1
        }

        fn active_core(&self) -> usize {
            0
        }

        fn set_breakpoint(&mut self, _addr: u32, _core: Option<usize>) -> anyhow::Result<BpId> {
            Ok(0)
        }

        fn clear_breakpoint(&mut self, _id: BpId) -> anyhow::Result<()> {
            Ok(())
        }

        fn set_watchpoint(
            &mut self,
            _addr: u32,
            _len: u32,
            _kind: WatchKind,
        ) -> anyhow::Result<WpId> {
            Ok(0)
        }

        fn clear_watchpoint(&mut self, _id: WpId) -> anyhow::Result<()> {
            Ok(())
        }

        fn read_mem(
            &mut self,
            _addr: u32,
            _len: u32,
            _core: Option<usize>,
        ) -> anyhow::Result<Vec<u8>> {
            Ok(Vec::new())
        }

        fn write_mem(
            &mut self,
            _addr: u32,
            _data: &[u8],
            _core: Option<usize>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        fn read_regs(&mut self, _core: Option<usize>) -> anyhow::Result<HashMap<String, u64>> {
            Ok(HashMap::new())
        }

        fn is_halted(&mut self, _core: Option<usize>) -> bool {
            !self.state.lock().unwrap().running
        }
    }

    fn chip() -> ChipConfig {
        ChipConfig {
            name: "mock-chip".into(),
            architecture: "cortex-m4".into(),
            flash_base: 0x0800_0000,
            flash_size: 512 * 1024,
            ram_base: 0x2000_0000,
            ram_size: 128 * 1024,
        }
    }

    fn mock(state: Arc<Mutex<MockState>>) -> Box<dyn DebugProbe> {
        Box::new(MockProbe { state })
    }

    #[test]
    fn attach_halts_running_target_before_returning_halted_session() {
        let state = Arc::new(Mutex::new(MockState {
            recover_connected: true,
            ..MockState::default()
        }));

        let session = Session::attach(&chip(), mock(state.clone())).unwrap();

        assert_eq!(session.state, SessionState::Halted);
        let state = state.lock().unwrap();
        assert!(!state.running);
        assert_eq!(state.calls, ["attach", "is_connected", "halt"]);
        assert!(!state.calls.contains(&"resume"));
    }

    #[test]
    fn attach_recovers_only_when_disconnected_before_halting() {
        let state = Arc::new(Mutex::new(MockState::default()));

        let session = Session::attach(&chip(), mock(state.clone())).unwrap();

        assert_eq!(session.state, SessionState::Halted);
        let state = state.lock().unwrap();
        assert_eq!(
            state.calls,
            [
                "attach",
                "is_connected",
                "try_recover",
                "is_connected",
                "halt"
            ]
        );
        assert!(!state.running);
        assert!(!state.calls.contains(&"resume"));
    }

    #[test]
    fn attach_failure_detaches_and_returns_error() {
        let state = Arc::new(Mutex::new(MockState {
            attach_error: true,
            ..MockState::default()
        }));

        let error = match Session::attach(&chip(), mock(state.clone())) {
            Ok(_) => panic!("attach unexpectedly succeeded"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("failed to attach debug backend"));
        assert_eq!(state.lock().unwrap().calls, ["attach", "detach"]);
    }

    #[test]
    fn initial_halt_failure_detaches_and_returns_error() {
        let state = Arc::new(Mutex::new(MockState {
            halt_error: true,
            recover_connected: true,
            ..MockState::default()
        }));

        let error = match Session::attach(&chip(), mock(state.clone())) {
            Ok(_) => panic!("attach unexpectedly succeeded"),
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains("failed to halt target after attach")
        );
        assert_eq!(
            state.lock().unwrap().calls,
            ["attach", "is_connected", "halt", "detach"]
        );
    }
}
