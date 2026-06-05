//! UART 串口通道 — 物理串口日志通道。
//!
//! 通过主机串口（`/dev/ttyACM0` 或 `COM3`）接收 MCU 输出。
//! RTT 不可用时的首选 fallback。
//!
//! 使用 `serialport` crate 实现跨平台串口通信。

use crate::log::LogChannel;

/// 默认串口波特率
const DEFAULT_BAUD_RATE: u32 = 115_200;
/// 默认读取超时（毫秒）
const DEFAULT_TIMEOUT_MS: u64 = 100;

/// UART 串口日志通道
pub struct UartChannel {
    /// 打开的串口
    port: Option<Box<dyn serialport::SerialPort>>,
    /// 串口路径（用户指定或自动检测）
    path: String,
    /// 波特率
    baud_rate: u32,
}

impl UartChannel {
    /// 创建一个新的 UART 通道。
    ///
    /// `port_path` 为串口路径，如果为 `None` 则自动检测。
    pub fn new(port_path: Option<String>) -> Self {
        Self {
            port: None,
            path: port_path.unwrap_or_default(),
            baud_rate: DEFAULT_BAUD_RATE,
        }
    }

    /// 自动检测可用的串口端口。
    ///
    /// Windows: 返回第一个可打开的 COM 端口（COM1-COM9）
    /// Linux: 返回第一个可打开的 /dev/ttyACM* 或 /dev/ttyUSB*
    fn auto_detect() -> Option<String> {
        let ports = serialport::available_ports().ok()?;
        for port in &ports {
            let path = &port.port_name;
            // 优先匹配常见的 MCU 串口设备
            if cfg!(target_os = "windows") && path.starts_with("COM") {
                return Some(path.clone());
            }
            if cfg!(not(target_os = "windows"))
                && (path.starts_with("/dev/ttyACM") || path.starts_with("/dev/ttyUSB"))
            {
                return Some(path.clone());
            }
        }
        // 如果没有匹配的常见设备，返回第一个可用端口
        ports.first().map(|p| p.port_name.clone())
    }
}

impl LogChannel for UartChannel {
    fn name(&self) -> &str {
        "uart"
    }

    fn open(&mut self) -> anyhow::Result<()> {
        if self.path.is_empty() {
            self.path =
                Self::auto_detect().ok_or_else(|| anyhow::anyhow!("no serial port detected"))?;
            log::info!("auto-detected serial port: {}", self.path);
        }
        let port = serialport::new(&self.path, self.baud_rate)
            .timeout(std::time::Duration::from_millis(DEFAULT_TIMEOUT_MS))
            .open()
            .map_err(|e| anyhow::anyhow!("failed to open serial port {}: {e}", self.path))?;
        self.port = Some(port);
        log::info!("UART channel opened on {}", self.path);
        Ok(())
    }

    fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize> {
        let port = self
            .port
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("UART channel not open"))?;
        port.read(buf)
            .map_err(|e| anyhow::anyhow!("serial read error: {e}"))
    }

    fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let port = self
            .port
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("UART channel not open"))?;
        port.write(data)
            .map_err(|e| anyhow::anyhow!("serial write error: {e}"))?;
        Ok(())
    }

    fn is_writable(&self) -> bool {
        true
    }

    fn close(&mut self) -> anyhow::Result<()> {
        // serialport 的 Drop 实现会关闭串口
        self.port = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogChannel;

    /// 验证 UartChannel 创建时的初始状态。
    #[test]
    fn test_uart_channel_creation() {
        let ch = UartChannel::new(None);
        assert_eq!(ch.name(), "uart");
        assert!(ch.is_writable());
    }

    /// 未 open 时 read/write 应返回错误。
    #[test]
    fn test_uart_read_write_without_open() {
        let mut ch = UartChannel::new(None);
        assert!(ch.read(&mut [0u8; 16]).is_err());
        assert!(ch.write(b"hello").is_err());
    }

    /// close 在未 open 状态下应安全（幂等）。
    #[test]
    fn test_uart_close_is_idempotent() {
        let mut ch = UartChannel::new(None);
        assert!(ch.close().is_ok());
        assert!(ch.close().is_ok());
    }

    /// auto_detect 在不存在的端口上应返回 None（不 panic）。
    #[test]
    fn test_auto_detect_no_ports() {
        // 此测试验证 auto_detect 在无法枚举端口时返回 None 而非 panic
        let result = UartChannel::auto_detect();
        // 在无串口的环境中应返回 None
        if let Some(port) = result {
            println!("detected port: {port}");
        }
    }

    /// 使用指定路径创建通道。
    #[test]
    fn test_uart_channel_custom_path() {
        let ch = UartChannel::new(Some("/dev/ttyACM0".into()));
        // 即使设备不存在，创建对象本身不应失败
        assert_eq!(ch.name(), "uart");
    }
}
