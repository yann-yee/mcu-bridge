//! UART 串口通道 — 物理串口日志通道。
//!
//! 通过主机串口 (`/dev/ttyACM0` 或 `COM3`) 接收 MCU 输出。
//! RTT 不可用时的首选 fallback。

use crate::log::LogChannel;

/// UART 串口日志通道
pub struct UartChannel;

impl LogChannel for UartChannel {
    fn name(&self) -> &str {
        "uart"
    }

    fn open(&mut self) -> anyhow::Result<()> {
        todo!("UartChannel: open serial port (auto-detect or user-specified)")
    }

    fn read(&mut self, _buf: &mut [u8]) -> anyhow::Result<usize> {
        todo!("UartChannel: blocking read from serial port")
    }

    fn write(&mut self, _data: &[u8]) -> anyhow::Result<()> {
        todo!("UartChannel: write to serial port")
    }

    fn is_writable(&self) -> bool {
        true
    }

    fn close(&mut self) -> anyhow::Result<()> {
        todo!("UartChannel: close serial port")
    }
}
