//! OpenOCD backend — TCL telnet 子进程，兜底后端。
//!
//! 以子进程模式启动 `openocd`，通过 `localhost:6666` TCL telnet 接口通信。
//! 覆盖 probe-rs 不支持的非主流芯片（冷门 Cortex-M、RISC-V、Xtensa 等）。

use crate::config::{ChipConfig, FlashOpts};
use crate::probe::{BpId, WpId};
use crate::probe::{DebugProbe, WatchKind};
use std::collections::HashMap;
use std::path::Path;

/// OpenOCD 后端实现
pub struct OpenOcdBackend;

impl DebugProbe for OpenOcdBackend {
    fn attach(&mut self, _chip: &ChipConfig) -> anyhow::Result<()> {
        todo!("OpenOCD attach: spawn openocd subprocess, wait for telnet port 6666")
    }

    fn detach(&mut self) -> anyhow::Result<()> {
        todo!("OpenOCD detach: shutdown telnet, kill subprocess")
    }

    fn is_connected(&self) -> bool {
        todo!("OpenOCD is_connected: check telnet socket")
    }

    fn try_recover(&mut self) -> anyhow::Result<()> {
        todo!("OpenOCD try_recover: kill + restart subprocess")
    }

    fn flash(&mut self, _elf: &Path, _opts: &FlashOpts) -> anyhow::Result<()> {
        todo!("OpenOCD flash: TCL 'program' command")
    }

    fn halt(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        todo!("OpenOCD halt: TCL 'halt'")
    }

    fn resume(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        todo!("OpenOCD resume: TCL 'resume'")
    }

    fn step(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        todo!("OpenOCD step: TCL 'step'")
    }

    fn core_count(&self) -> usize {
        todo!("OpenOCD core_count")
    }

    fn active_core(&self) -> usize {
        todo!("OpenOCD active_core")
    }

    fn set_breakpoint(&mut self, _addr: u32, _core: Option<usize>) -> anyhow::Result<BpId> {
        todo!("OpenOCD set_breakpoint: TCL 'bp'")
    }

    fn clear_breakpoint(&mut self, _id: BpId) -> anyhow::Result<()> {
        todo!("OpenOCD clear_breakpoint: TCL 'rbp'")
    }

    fn set_watchpoint(&mut self, _addr: u32, _len: u32, _kind: WatchKind) -> anyhow::Result<WpId> {
        todo!("OpenOCD set_watchpoint: TCL 'wp'")
    }

    fn clear_watchpoint(&mut self, _id: WpId) -> anyhow::Result<()> {
        todo!("OpenOCD clear_watchpoint: TCL 'rwp'")
    }

    fn read_mem(&mut self, _addr: u32, _len: u32, _core: Option<usize>) -> anyhow::Result<Vec<u8>> {
        todo!("OpenOCD read_mem: TCL 'mdw'")
    }

    fn write_mem(&mut self, _addr: u32, _data: &[u8], _core: Option<usize>) -> anyhow::Result<()> {
        todo!("OpenOCD write_mem: TCL 'mww'")
    }

    fn read_regs(&mut self, _core: Option<usize>) -> anyhow::Result<HashMap<String, u64>> {
        todo!("OpenOCD read_regs: TCL 'reg'")
    }

    fn is_halted(&self, _core: Option<usize>) -> bool {
        todo!("OpenOCD is_halted")
    }
}
