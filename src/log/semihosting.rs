//! Semihosting 通道 — 三级 fallback 的最后一级。
//!
//! MCU 通过 `BKPT` 异常陷入调试器来输出文本。
//! 每次输出约 1-2ms 且会 halt CPU，性能差但无需任何硬件连接。
//! 只读不可写，不做性能优化（协议固有缺陷）。
//!
//! P2 备注: probe-rs 0.31 不暴露直接的 semihosting 读 API，
//! 此实现作为扩展点保留。真正的数据捕获需 probe-rs 未来版本支持。

use std::sync::{Arc, Mutex};

use crate::probe::DebugProbe;

/// Semihosting 日志通道
pub struct SemihostingChannel {
    /// 共享后端引用
    backend: Arc<Mutex<Box<dyn DebugProbe>>>,
    /// Semihosting 是否已启用
    enabled: bool,
}

impl SemihostingChannel {
    /// 创建一个新的 Semihosting 通道。
    pub fn new(backend: Arc<Mutex<Box<dyn DebugProbe>>>) -> Self {
        Self {
            backend,
            enabled: false,
        }
    }
}

impl crate::log::LogChannel for SemihostingChannel {
    fn name(&self) -> &str {
        "semihosting"
    }

    fn open(&mut self) -> anyhow::Result<()> {
        let mut guard = self
            .backend
            .lock()
            .map_err(|e| anyhow::anyhow!("backend lock poisoned: {e}"))?;
        guard.enable_semihosting()?;
        self.enabled = true;
        log::info!("Semihosting channel enabled");
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize> {
        if !self.enabled {
            anyhow::bail!("Semihosting channel not open");
        }
        let mut guard = self
            .backend
            .lock()
            .map_err(|e| anyhow::anyhow!("backend lock poisoned: {e}"))?;
        guard.read_semihosting(buf)
    }

    fn write(&mut self, _data: &[u8]) -> anyhow::Result<()> {
        anyhow::bail!("Semihosting is read-only");
    }

    fn is_writable(&self) -> bool {
        false
    }

    fn close(&mut self) -> anyhow::Result<()> {
        self.enabled = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogChannel;
    use crate::probe::probe_rs::ProbeRsBackend;
    use std::sync::{Arc, Mutex};

    /// 验证 SemihostingChannel 创建时的初始状态。
    #[test]
    fn test_semihosting_channel_creation() {
        let backend: Box<dyn DebugProbe> = Box::new(ProbeRsBackend::new());
        let backend = Arc::new(Mutex::new(backend));
        let ch = SemihostingChannel::new(backend);
        assert_eq!(ch.name(), "semihosting");
        assert!(!ch.is_writable());
    }

    /// 未 open 时 read 应返回错误。
    #[test]
    fn test_semihosting_read_without_open() {
        let backend: Box<dyn DebugProbe> = Box::new(ProbeRsBackend::new());
        let backend = Arc::new(Mutex::new(backend));
        let mut ch = SemihostingChannel::new(backend);
        assert!(ch.read(&mut [0u8; 16]).is_err());
    }

    /// write 始终返回错误（Semihosting 只读）。
    #[test]
    fn test_semihosting_write_always_fails() {
        let backend: Box<dyn DebugProbe> = Box::new(ProbeRsBackend::new());
        let backend = Arc::new(Mutex::new(backend));
        let mut ch = SemihostingChannel::new(backend);
        let result = ch.write(b"hello");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("read-only"));
    }

    /// close 在未 open 状态下应安全（幂等）。
    #[test]
    fn test_semihosting_close_is_idempotent() {
        let backend: Box<dyn DebugProbe> = Box::new(ProbeRsBackend::new());
        let backend = Arc::new(Mutex::new(backend));
        let mut ch = SemihostingChannel::new(backend);
        assert!(ch.close().is_ok());
        assert!(ch.close().is_ok());
    }
}
