/// 调试探针抽象层 — 屏蔽 probe-rs 与 OpenOCD 后端差异。
///
/// 设计文档 §3.1 定义了 17 个方法的统一 trait。
/// 上层（Session / DebugBuffer / LogChannel / CLI）只依赖此 trait，
/// 完全不感知底层后端实现。
use std::collections::HashMap;
use std::path::Path;

use crate::config::{ChipConfig, FlashOpts};

pub mod openocd;
pub mod probe_rs;

/// 断点标识符
pub type BpId = usize;

/// 数据观测点标识符
pub type WpId = usize;

/// 观测触发类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchKind {
    /// 仅读触发
    Read,
    /// 仅写触发（P2 预留）
    #[allow(dead_code)]
    Write,
    /// 读写均触发（默认，P2 预留）
    #[allow(dead_code)]
    ReadWrite,
}

/// 调试探针抽象 trait。
///
/// 所有方法均可失败返回 `anyhow::Error`。
/// `core` 参数默认为 `None`（活跃核），显式传 `Some(n)` 指定目标核。
///
/// ⚠ 部分方法是 P2/P3 预留，当前未从 trait 调用路径触发 dead_code 警告。
#[allow(dead_code)]
pub trait DebugProbe: Send {
    // ── 会话生命周期 ──

    /// 连接到目标芯片
    fn attach(&mut self, chip: &ChipConfig) -> anyhow::Result<()>;

    /// 断开连接
    fn detach(&mut self) -> anyhow::Result<()>;

    // ── 连接自恢复 ──

    /// 探针是否在线
    fn is_connected(&self) -> bool;

    /// 尝试恢复连接（probe-rs 有内置重枚举能力）
    fn try_recover(&mut self) -> anyhow::Result<()>;

    // ── 烧录 ──

    /// 将 ELF 固件烧录到目标 Flash
    fn flash(&mut self, elf: &Path, opts: &FlashOpts) -> anyhow::Result<()>;

    // ── 执行控制 ──

    /// 暂停指定核（None = 活跃核）
    fn halt(&mut self, core: Option<usize>) -> anyhow::Result<()>;

    /// 全速运行指定核
    fn resume(&mut self, core: Option<usize>) -> anyhow::Result<()>;

    /// 复位指定核（复位后继续执行）
    ///
    /// 默认实现：halt → 复位向量 → resume（等效于冷启动）。
    /// 后端可覆盖以实现硬件复位信号（nRST）或 SoC 级复位。
    fn reset(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        anyhow::bail!("reset not supported by this backend")
    }

    /// 单步（源码行级，需 DWARF）
    fn step(&mut self, core: Option<usize>) -> anyhow::Result<()>;

    // ── 多核 ──

    /// 探针检测到的核数
    fn core_count(&self) -> usize;

    /// 当前活跃核编号
    fn active_core(&self) -> usize;

    // ── 断点 ──

    /// 设置硬件断点，返回断点 ID
    fn set_breakpoint(&mut self, addr: u32, core: Option<usize>) -> anyhow::Result<BpId>;

    /// 清除指定断点
    fn clear_breakpoint(&mut self, id: BpId) -> anyhow::Result<()>;

    // ── 数据观测 ──

    /// 设置硬件 watchpoint
    fn set_watchpoint(&mut self, addr: u32, len: u32, kind: WatchKind) -> anyhow::Result<WpId>;

    /// 清除指定 watchpoint
    fn clear_watchpoint(&mut self, id: WpId) -> anyhow::Result<()>;

    // ── 内存与寄存器 ──

    /// 读取内存
    fn read_mem(&mut self, addr: u32, len: u32, core: Option<usize>) -> anyhow::Result<Vec<u8>>;

    /// 写入内存
    fn write_mem(&mut self, addr: u32, data: &[u8], core: Option<usize>) -> anyhow::Result<()>;

    /// 读取指定核的寄存器快照
    fn read_regs(&mut self, core: Option<usize>) -> anyhow::Result<HashMap<String, u64>>;

    // ── 状态 ──

    /// 指定核是否处于 halted 状态
    fn is_halted(&mut self, core: Option<usize>) -> bool;

    // ── RTT (Real-Time Transfer) ──

    /// 搜索并附着 RTT Control Block。
    ///
    /// 成功后可通过 `rtt_read` / `rtt_write` 与 MCU 交换数据。
    /// 默认返回错误（不支持 RTT 的后端）。
    fn rtt_attach(&mut self, _core_idx: usize) -> anyhow::Result<()> {
        anyhow::bail!("RTT not supported by this backend")
    }

    /// RTT 是否已附着。
    fn rtt_is_attached(&self) -> bool {
        false
    }

    /// 从指定 RTT up channel 读取数据。
    fn rtt_read(&mut self, _channel: usize, _buf: &mut [u8]) -> anyhow::Result<usize> {
        anyhow::bail!("RTT not supported by this backend")
    }

    /// 向指定 RTT down channel 写入数据。
    fn rtt_write(&mut self, _channel: usize, _data: &[u8]) -> anyhow::Result<usize> {
        anyhow::bail!("RTT not supported by this backend")
    }

    /// 断开 RTT 连接。
    fn rtt_detach(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    // ── Semihosting ──

    /// 启用 ARM Semihosting 支持。
    fn enable_semihosting(&mut self) -> anyhow::Result<()> {
        anyhow::bail!("Semihosting not supported by this backend")
    }

    /// 读取 Semihosting 输出数据。
    fn read_semihosting(&mut self, _buf: &mut [u8]) -> anyhow::Result<usize> {
        anyhow::bail!("Semihosting not supported by this backend")
    }

    /// Semihosting 是否已启用。
    fn is_semihosting_enabled(&self) -> bool {
        false
    }

    /// 轮询目标是否已 halt（更新内部缓存状态）。
    ///
    /// 默认实现直接返回 `is_halted()`。
    /// probe-rs 后端在此方法中用 `wait_for_core_halted` 短超时轮询更新状态缓存。
    fn poll_halted(&mut self, core: Option<usize>) -> bool {
        self.is_halted(core)
    }
}
