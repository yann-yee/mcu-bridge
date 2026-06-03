//! probe-rs backend — 纯 Rust API 直驱，默认后端。
//!
//! 支持 CMSIS-DAP / J-Link / ST-Link 等探针。
//! 通过 `probe-rs` crate 直接驱动，无需外部进程。

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use probe_rs::MemoryInterface;

use crate::config::{ChipConfig, FlashOpts};
use crate::probe::{BpId, WpId};
use crate::probe::{DebugProbe, WatchKind};

/// probe-rs 后端实现
pub struct ProbeRsBackend {
    /// probe-rs Session，attach 后持有，detach 时释放
    session: Option<probe_rs::Session>,
    /// 当前活跃核编号
    active_core: usize,
    /// 探针检测到的核数（attach 时记录）
    num_cores: usize,
    /// 断点 ID 计数器
    next_bp_id: BpId,
    /// 断点地址 → ID 映射（用于 clear_breakpoint 反查）
    bp_map: HashMap<u64, BpId>,
    /// watchpoint ID 计数器
    next_wp_id: WpId,
}

impl ProbeRsBackend {
    /// 创建一个新的 probe-rs 后端（未连接状态）
    pub fn new() -> Self {
        Self {
            session: None,
            active_core: 0,
            num_cores: 0,
            next_bp_id: 0,
            bp_map: HashMap::new(),
            next_wp_id: 0,
        }
    }

    /// 获取指定核的 mutable 引用。
    fn get_core(&mut self, core: Option<usize>) -> anyhow::Result<probe_rs::Core<'_>> {
        let idx = core.unwrap_or(self.active_core);
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("not attached"))?;
        Ok(s.core(idx)?)
    }
}

impl Default for ProbeRsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugProbe for ProbeRsBackend {
    fn attach(&mut self, chip: &ChipConfig) -> anyhow::Result<()> {
        let target = probe_rs::config::TargetSelector::Unspecified(chip.name.clone());
        let config = probe_rs::SessionConfig {
            permissions: probe_rs::Permissions::new(),
            speed: None,
            protocol: None,
        };
        let session = probe_rs::Session::auto_attach(target, config)
            .map_err(|e| anyhow::anyhow!("probe-rs attach failed: {e}"))?;
        self.num_cores = session.list_cores().len();
        self.session = Some(session);
        self.active_core = 0;
        Ok(())
    }

    fn detach(&mut self) -> anyhow::Result<()> {
        self.session = None;
        self.bp_map.clear();
        self.num_cores = 0;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        // P1 实现 — probe-rs Session 不提供在线检测 API，
        // 需尝试一次轻量操作判断。P0 暂返回 session 是否存在。
        self.session.is_some()
    }

    fn try_recover(&mut self) -> anyhow::Result<()> {
        anyhow::bail!("P1: probe recovery not yet implemented")
    }

    fn flash(&mut self, elf: &Path, _opts: &FlashOpts) -> anyhow::Result<()> {
        let session = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("not attached"))?;
        probe_rs::flashing::download_file(
            session,
            elf,
            probe_rs::flashing::Format::Elf(probe_rs::flashing::ElfOptions::default()),
        )
        .map_err(|e| anyhow::anyhow!("flash failed: {e}"))?;
        Ok(())
    }

    fn halt(&mut self, core: Option<usize>) -> anyhow::Result<()> {
        let mut core = self.get_core(core)?;
        core.halt(Duration::from_millis(500))
            .map_err(|e| anyhow::anyhow!("halt failed: {e}"))?;
        Ok(())
    }

    fn resume(&mut self, core: Option<usize>) -> anyhow::Result<()> {
        let mut core = self.get_core(core)?;
        core.run()
            .map_err(|e| anyhow::anyhow!("resume failed: {e}"))?;
        Ok(())
    }

    fn step(&mut self, core: Option<usize>) -> anyhow::Result<()> {
        let mut core = self.get_core(core)?;
        core.step()
            .map_err(|e| anyhow::anyhow!("step failed: {e}"))?;
        Ok(())
    }

    fn core_count(&self) -> usize {
        self.num_cores
    }

    fn active_core(&self) -> usize {
        self.active_core
    }

    fn set_breakpoint(&mut self, addr: u32, core: Option<usize>) -> anyhow::Result<BpId> {
        let addr = addr as u64;
        // 先分配 ID、再借 core
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        self.bp_map.insert(addr, id);
        // 操作 core
        let result = {
            let mut core = self.get_core(core)?;
            core.set_hw_breakpoint(addr)
        };
        match result {
            Ok(()) => Ok(id),
            Err(e) => {
                // 回滚
                self.bp_map.remove(&addr);
                self.next_bp_id -= 1;
                Err(anyhow::anyhow!(
                    "set breakpoint at 0x{addr:08x} failed: {e}"
                ))
            }
        }
    }

    fn clear_breakpoint(&mut self, id: BpId) -> anyhow::Result<()> {
        // 从 bp_map 反查地址，避免借用冲突
        let addr = self
            .bp_map
            .iter()
            .find(|&(_, bid)| *bid == id)
            .map(|(&a, _)| a)
            .ok_or_else(|| anyhow::anyhow!("breakpoint #{id} not found"))?;
        let s = self
            .session
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("not attached"))?;
        let active = self.active_core;
        let mut core = s.core(active)?;
        core.clear_hw_breakpoint(addr)
            .map_err(|e| anyhow::anyhow!("clear breakpoint #{id} at 0x{addr:08x} failed: {e}"))?;
        self.bp_map.retain(|&a, _| a != addr);
        Ok(())
    }

    fn set_watchpoint(&mut self, _addr: u32, _len: u32, _kind: WatchKind) -> anyhow::Result<WpId> {
        anyhow::bail!("P1: watchpoint not yet implemented")
    }

    fn clear_watchpoint(&mut self, _id: WpId) -> anyhow::Result<()> {
        anyhow::bail!("P1: watchpoint not yet implemented")
    }

    fn read_mem(&mut self, addr: u32, len: u32, core: Option<usize>) -> anyhow::Result<Vec<u8>> {
        let mut core = self.get_core(core)?;
        let mut buf = Vec::with_capacity(len as usize);
        let mut current = addr as u64;
        let end = current + len as u64;
        while current < end {
            let word = core
                .read_word_32(current)
                .map_err(|e| anyhow::anyhow!("read_mem at 0x{current:08x} failed: {e}"))?;
            buf.extend_from_slice(&word.to_le_bytes());
            current += 4;
        }
        buf.truncate(len as usize);
        Ok(buf)
    }

    fn write_mem(&mut self, addr: u32, data: &[u8], core: Option<usize>) -> anyhow::Result<()> {
        let mut core = self.get_core(core)?;
        let chunks = data.chunks(4);
        let mut current = addr as u64;
        for chunk in chunks {
            let mut word_bytes = [0u8; 4];
            word_bytes[..chunk.len()].copy_from_slice(chunk);
            let word = u32::from_le_bytes(word_bytes);
            core.write_word_32(current, word)
                .map_err(|e| anyhow::anyhow!("write_mem at 0x{current:08x} failed: {e}"))?;
            current += 4;
        }
        Ok(())
    }

    fn read_regs(&mut self, core: Option<usize>) -> anyhow::Result<HashMap<String, u64>> {
        let mut core = self.get_core(core)?;
        let registers = core.registers();
        let mut map = HashMap::new();
        for reg in registers.core_registers() {
            let value: probe_rs::RegisterValue = core
                .read_core_reg(reg.id())
                .map_err(|e| anyhow::anyhow!("read reg {} failed: {e}", reg.name()))?;
            let val_u64: u64 = value.try_into().unwrap_or(0);
            map.insert(reg.name().to_string(), val_u64);
        }
        Ok(map)
    }

    fn is_halted(&self, _core: Option<usize>) -> bool {
        // P1 实现 — 需调用 core.core_halted()
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 验证后端创建时的初始状态。
    #[test]
    fn test_backend_creation() {
        let b = ProbeRsBackend::new();
        assert!(!b.is_connected());
        assert_eq!(b.core_count(), 0);
        assert_eq!(b.active_core(), 0);
    }

    /// 无硬件时 attach 应返回 Err 而不是 panic。
    #[test]
    fn test_attach_without_hardware() {
        let mut b = ProbeRsBackend::new();
        let chip = ChipConfig {
            name: "STM32F407VG".into(),
            architecture: "cortex-m4".into(),
            flash_base: 0x08000000,
            flash_size: 0x100000,
            ram_base: 0x20000000,
            ram_size: 0x20000,
        };
        let result = b.attach(&chip);
        assert!(result.is_err());
    }

    /// detach 在未连接状态下应是安全的（幂等）。
    #[test]
    fn test_detach_is_idempotent() {
        let mut b = ProbeRsBackend::new();
        assert!(b.detach().is_ok());
        assert!(b.detach().is_ok());
    }

    /// 未连接时 halt 应返回 Err 并包含 "not attached"。
    #[test]
    fn test_halt_without_attach() {
        let mut b = ProbeRsBackend::new();
        let err = b.halt(None).unwrap_err();
        assert!(err.to_string().contains("not attached"));
    }

    /// 未连接时 resume 应返回 Err。
    #[test]
    fn test_resume_without_attach() {
        let mut b = ProbeRsBackend::new();
        assert!(b.resume(None).is_err());
    }

    /// 未连接时 step 应返回 Err。
    #[test]
    fn test_step_without_attach() {
        let mut b = ProbeRsBackend::new();
        assert!(b.step(None).is_err());
    }

    /// 未连接时 read_mem 应返回 Err。
    #[test]
    fn test_read_mem_without_attach() {
        let mut b = ProbeRsBackend::new();
        assert!(b.read_mem(0x08000000, 16, None).is_err());
    }

    /// Default::default() 应产生空后端。
    #[test]
    fn test_default_creates_empty_backend() {
        let b: ProbeRsBackend = Default::default();
        assert_eq!(b.core_count(), 0);
    }

    /// P1 预留方法应返回 Err 而非 panic。
    #[test]
    fn test_p1_methods_return_error() {
        let mut b = ProbeRsBackend::new();
        assert!(b.set_watchpoint(0, 4, WatchKind::ReadWrite).is_err());
        assert!(b.clear_watchpoint(0).is_err());
        assert!(b.try_recover().is_err());
        assert!(!b.is_halted(None));
    }

    /// 断点 ID 分配在未连接时不应 panic。
    #[test]
    fn test_breakpoint_id_counter() {
        let mut b = ProbeRsBackend::new();
        assert_eq!(b.next_bp_id, 0);
        let chip = ChipConfig {
            name: "nRF52840".into(),
            architecture: "cortex-m4".into(),
            flash_base: 0,
            flash_size: 0x100000,
            ram_base: 0x20000000,
            ram_size: 0x40000,
        };
        let _ = b.attach(&chip);
        assert_eq!(b.next_bp_id, 0);
    }
}
