/// 日志通道抽象层 — RTT / UART / Semihosting 统一为 MCU↔主机文本字节流。
///
/// 设计文档 §3.3 定义了 6 个方法的 trait。
/// `SerialMonitor` 线程持有 `Box<dyn LogChannel>` 持续读取日志数据。
pub mod rtt;
pub mod semihosting;
pub mod uart;

use std::sync::{Arc, Mutex};

use crate::probe::DebugProbe;

/// 日志通道 trait。
///
/// 所有实现必须同时实现 `Send`。
/// `write` 和 `is_writable` 当前未从调用链触达（P2 扩展点）。
#[allow(dead_code)]
pub trait LogChannel: Send {
    /// 通道名称: "rtt" | "uart" | "semihosting"
    fn name(&self) -> &str;

    /// 打开通道（建立连接）
    fn open(&mut self) -> anyhow::Result<()>;

    /// 从通道读取数据，返回读取的字节数
    fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize>;

    /// 向通道写入数据
    fn write(&mut self, data: &[u8]) -> anyhow::Result<()>;

    /// 通道是否可写（Semihosting 只读不可写）
    fn is_writable(&self) -> bool;

    /// 关闭通道
    fn close(&mut self) -> anyhow::Result<()>;
}

/// 三级 fallback 日志通道检测。
///
/// 按优先级依次尝试：
///   1. RTT（首选，probe-rs 原生）
///   2. UART（第一 fallback，物理串口）
///   3. Semihosting（最终 fallback，BKPT 异常）
///
/// 所有通道均失败时返回 `None` —— 日志通道是可选能力，不阻止会话继续。
///
/// # 参数
/// * `backend` — 共享后端引用（RTT/Semihosting 通道需要）
/// * `serial_port` — 用户指定的串口端口（仅 UART 通道使用，None = 自动检测）
/// * `core_idx` — 目标核编号（RTT/Semihosting 通道附着使用）
pub fn detect_log_backend(
    backend: Arc<Mutex<Box<dyn DebugProbe>>>,
    serial_port: Option<String>,
    core_idx: usize,
) -> Option<Box<dyn LogChannel>> {
    // 1. 尝试 RTT
    {
        let mut ch = rtt::RttChannel::new(backend.clone(), core_idx, 0, 0);
        if ch.open().is_ok() {
            log::info!("log backend: RTT channel active on core {core_idx}");
            return Some(Box::new(ch));
        }
        log::debug!("RTT not available, falling back to UART");
    }

    // 2. 尝试 UART
    {
        let mut ch = uart::UartChannel::new(serial_port);
        if ch.open().is_ok() {
            log::info!("log backend: UART channel active");
            return Some(Box::new(ch));
        }
        log::debug!("UART not available, falling back to Semihosting");
    }

    // 3. 尝试 Semihosting
    {
        let mut ch = semihosting::SemihostingChannel::new(backend);
        if ch.open().is_ok() {
            log::info!("log backend: Semihosting channel active");
            return Some(Box::new(ch));
        }
        log::warn!("all log backends failed");
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::probe_rs::ProbeRsBackend;

    /// 验证 detect_log_backend 在无硬件时返回 None（而非 panic）。
    #[test]
    fn test_detect_fallback_all_fail() {
        let backend: Box<dyn DebugProbe> = Box::new(ProbeRsBackend::new());
        let backend = Arc::new(Mutex::new(backend));
        let result = detect_log_backend(backend, None, 0);
        assert!(
            result.is_none(),
            "should return None when no backend available"
        );
    }

    /// 验证 detect_log_backend 的返回值类型。
    #[test]
    fn test_detect_fallback_type_safety() {
        let backend: Box<dyn DebugProbe> = Box::new(ProbeRsBackend::new());
        let backend = Arc::new(Mutex::new(backend));
        let result = detect_log_backend(backend, None, 0);
        // 类型安全：返回 Option<Box<dyn LogChannel>>，编译即证明类型正确
        assert!(result.is_none());
    }
}
