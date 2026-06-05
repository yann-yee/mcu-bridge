//! RTT (SEGGER Real-Time Transfer) 通道 — 首选日志通道。
//!
//! MCU 侧仅 memcpy，调试器通过 SWD 直接读取 RAM 中的 RTT Control Block。
//! 启动时通过 probe-rs 的 `Rtt::attach()` 搜索 `"SEGGER RTT"` 魔数来检测。
//!
//! 实现方案 Q1=A: 使用 probe-rs 内置 Rtt API，不自行实现 CB 搜索。

use std::sync::{Arc, Mutex};

use crate::probe::DebugProbe;

/// RTT 日志通道
pub struct RttChannel {
    /// 共享后端引用
    backend: Arc<Mutex<Box<dyn DebugProbe>>>,
    /// RTT 附着的核编号
    core_idx: usize,
    /// 读取的 up channel 编号
    channel_up: usize,
    /// 写入的 down channel 编号（P2 扩展预留）
    #[allow(dead_code)]
    channel_down: usize,
    /// 是否已附着
    attached: bool,
}

impl RttChannel {
    /// 创建一个新的 RTT 通道。
    ///
    /// `backend` 为共享后端引用（与 Session/Sampler 共享）。
    /// `core_idx` 指定附着到哪个核。
    /// `channel_up` / `channel_down` 指定使用的 RTT 通道（通常为 0）。
    pub fn new(
        backend: Arc<Mutex<Box<dyn DebugProbe>>>,
        core_idx: usize,
        channel_up: usize,
        channel_down: usize,
    ) -> Self {
        Self {
            backend,
            core_idx,
            channel_up,
            channel_down,
            attached: false,
        }
    }
}

impl crate::log::LogChannel for RttChannel {
    fn name(&self) -> &str {
        "rtt"
    }

    fn open(&mut self) -> anyhow::Result<()> {
        let mut guard = self
            .backend
            .lock()
            .map_err(|e| anyhow::anyhow!("backend lock poisoned: {e}"))?;
        guard.rtt_attach(self.core_idx)?;
        self.attached = true;
        log::info!("RTT channel attached on core {}", self.core_idx);
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize> {
        if !self.attached {
            anyhow::bail!("RTT channel not open");
        }
        let mut guard = self
            .backend
            .lock()
            .map_err(|e| anyhow::anyhow!("backend lock poisoned: {e}"))?;
        guard.rtt_read(self.channel_up, buf)
    }

    fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        if !self.attached {
            anyhow::bail!("RTT channel not open");
        }
        let mut guard = self
            .backend
            .lock()
            .map_err(|e| anyhow::anyhow!("backend lock poisoned: {e}"))?;
        guard.rtt_write(self.channel_down, data)?;
        Ok(())
    }

    fn is_writable(&self) -> bool {
        true
    }

    fn close(&mut self) -> anyhow::Result<()> {
        if self.attached {
            let mut guard = self
                .backend
                .lock()
                .map_err(|e| anyhow::anyhow!("backend lock poisoned: {e}"))?;
            guard.rtt_detach()?;
            self.attached = false;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogChannel;
    use crate::probe::probe_rs::ProbeRsBackend;
    use std::sync::{Arc, Mutex};

    /// 验证 RttChannel 创建时的初始状态。
    #[test]
    fn test_rtt_channel_creation() {
        let backend: Box<dyn DebugProbe> = Box::new(ProbeRsBackend::new());
        let backend = Arc::new(Mutex::new(backend));
        let ch = RttChannel::new(backend, 0, 0, 0);
        assert_eq!(ch.name(), "rtt");
        assert!(ch.is_writable());
    }

    /// 无硬件时 open 应返回 Err（测试 RTT 后端未连接路径）。
    #[test]
    fn test_rtt_open_without_hardware() {
        let backend: Box<dyn DebugProbe> = Box::new(ProbeRsBackend::new());
        let backend = Arc::new(Mutex::new(backend));
        let mut ch = RttChannel::new(backend, 0, 0, 0);
        let result = ch.open();
        assert!(result.is_err(), "RTT attach without hardware should fail");
    }

    /// 未 open 时 read/write 应返回错误。
    #[test]
    fn test_rtt_read_write_without_open() {
        let backend: Box<dyn DebugProbe> = Box::new(ProbeRsBackend::new());
        let backend = Arc::new(Mutex::new(backend));
        let mut ch = RttChannel::new(backend, 0, 0, 0);
        assert!(ch.read(&mut [0u8; 16]).is_err());
        assert!(ch.write(b"hello").is_err());
    }

    /// close 在未 open 状态下应安全（幂等）。
    #[test]
    fn test_rtt_close_is_idempotent() {
        let backend: Box<dyn DebugProbe> = Box::new(ProbeRsBackend::new());
        let backend = Arc::new(Mutex::new(backend));
        let mut ch = RttChannel::new(backend, 0, 0, 0);
        assert!(ch.close().is_ok());
        assert!(ch.close().is_ok());
    }
}
