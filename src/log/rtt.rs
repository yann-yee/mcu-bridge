//! RTT (SEGGER Real-Time Transfer) 通道 — 首选日志通道。
//!
//! MCU 侧仅 memcpy，调试器通过 SWD 直接读取 RAM 中的 RTT Control Block。
//! 启动时搜索 `"SEGGER RTT"` 魔数来检测。

use crate::log::LogChannel;

/// RTT 日志通道
pub struct RttChannel;

impl LogChannel for RttChannel {
    fn name(&self) -> &str {
        "rtt"
    }

    fn open(&mut self) -> anyhow::Result<()> {
        todo!("RttChannel: search 'SEGGER RTT' signature in RAM, attach up channel 0")
    }

    fn read(&mut self, _buf: &mut [u8]) -> anyhow::Result<usize> {
        todo!("RttChannel: read from RTT up buffer via SWD")
    }

    fn write(&mut self, _data: &[u8]) -> anyhow::Result<()> {
        todo!("RttChannel: write to RTT down buffer via SWD")
    }

    fn is_writable(&self) -> bool {
        true
    }

    fn close(&mut self) -> anyhow::Result<()> {
        todo!("RttChannel: detach from RTT control block")
    }
}
