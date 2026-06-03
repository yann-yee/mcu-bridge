//! Semihosting 通道 — 三级 fallback 的最后一级。
//!
//! MCU 通过 `BKPT` 异常陷入调试器来输出文本。
//! 每次输出约 1-2ms 且会 halt CPU，性能差但无需任何硬件连接。
//! 只读不可写，不做性能优化（协议固有缺陷）。

use crate::log::LogChannel;

/// Semihosting 日志通道
pub struct SemihostingChannel;

impl LogChannel for SemihostingChannel {
    fn name(&self) -> &str {
        "semihosting"
    }

    fn open(&mut self) -> anyhow::Result<()> {
        todo!("SemihostingChannel: enable semihosting via SWD")
    }

    fn read(&mut self, _buf: &mut [u8]) -> anyhow::Result<usize> {
        todo!("SemihostingChannel: capture BKPT exception text")
    }

    fn write(&mut self, _data: &[u8]) -> anyhow::Result<()> {
        todo!("SemihostingChannel: write not supported — panic instead")
    }

    fn is_writable(&self) -> bool {
        // Semihosting 只读，不可写
        false
    }

    fn close(&mut self) -> anyhow::Result<()> {
        todo!("SemihostingChannel: disable semihosting")
    }
}
