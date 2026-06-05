//! OpenOCD backend — TCL telnet 子进程，兜底后端。
//!
//! 以子进程模式启动 `openocd`，通过 `localhost:6666` TCL telnet 接口通信。
//! 覆盖 probe-rs 不支持的非主流芯片（冷门 Cortex-M、RISC-V、Xtensa 等）。
//!
//! # Standalone flash 生命周期
//!
//! 1. [`attach()`] — spawn openocd + 轮询 TCP 6666 就绪
//! 2. [`flash()`] — 发送 `program <elf> verify` 命令
//! 3. [`resume()`] — (可选) 发送 `resume` 命令
//! 4. [`detach()`] — 发送 `exit` → wait 子进程 → kill 保底

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::config::{ChipConfig, FlashOpts};
use crate::probe::{BpId, WpId};
use crate::probe::{DebugProbe, WatchKind};

/// OpenOCD 后端实现
pub struct OpenOcdBackend {
    /// openocd 子进程
    process: Option<Child>,
    /// TCP localhost:6666 连接
    telnet: Option<TcpStream>,
    /// OpenOCD 配置文件路径（None 时尝试 .debugger/openocd.cfg）
    cfg_path: Option<String>,
    /// 目标是否处于 halted 状态（Q2=A 关键）
    target_halted: bool,
    /// 断点 ID 计数器
    next_bp_id: BpId,
    /// 断点地址 → ID 映射
    bp_map: HashMap<u64, BpId>,
    /// watchpoint ID 计数器（P2 预留）
    #[allow(dead_code)]
    next_wp_id: WpId,
    /// watchpoint 地址 → ID 映射（P2 预留）
    #[allow(dead_code)]
    wp_map: HashMap<u64, WpId>,
}

impl OpenOcdBackend {
    /// 创建一个新的 OpenOCD 后端。
    ///
    /// `cfg_path` 为可选的 OpenOCD 配置文件路径。
    /// 若为 `None`，`attach()` 时会尝试 `.debugger/openocd.cfg`。
    pub fn new(cfg_path: Option<String>) -> Self {
        Self {
            process: None,
            telnet: None,
            cfg_path,
            target_halted: false,
            next_bp_id: 0,
            bp_map: HashMap::new(),
            next_wp_id: 0,
            wp_map: HashMap::new(),
        }
    }

    /// 拼接 OpenOCD 命令行并启动子进程。
    fn spawn_openocd(&mut self, resolved_cfg: &str) -> anyhow::Result<()> {
        let child = Command::new("openocd")
            .args([
                "-f",
                resolved_cfg,
                "-c",
                "tcl_port 6666",
                "-c",
                "gdb_port disabled",
                "-c",
                "telnet_port disabled",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow::anyhow!("OpenOCD not found: {e}"))?;
        self.process = Some(child);
        Ok(())
    }

    /// OpenOCD TCL telnet 地址
    const TELNET_ADDR: SocketAddr = SocketAddr::new(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)),
        6666,
    );

    /// 等待 TCP localhost:6666 端口就绪，最多 5 秒。
    fn wait_for_telnet(&self) -> anyhow::Result<TcpStream> {
        let max_attempts = 25; // 25 × 200ms = 5s
        for attempt in 0..max_attempts {
            match TcpStream::connect_timeout(&Self::TELNET_ADDR, Duration::from_millis(200)) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
                    return Ok(stream);
                }
                Err(_) => {
                    if attempt == max_attempts - 1 {
                        anyhow::bail!("OpenOCD failed to start (timeout after 5s)");
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
        }
        anyhow::bail!("OpenOCD failed to start (timeout after 5s)");
    }

    /// 发送 TCL 命令并等待提示符返回，返回完整响应文本。
    fn tcl_command(&mut self, cmd: &str) -> anyhow::Result<String> {
        let stream = self
            .telnet
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("OpenOCD not connected"))?;

        // 发送命令
        let full_cmd = format!("{cmd}\r\n");
        stream
            .write_all(full_cmd.as_bytes())
            .map_err(|e| anyhow::anyhow!("OpenOCD write error: {e}"))?;
        stream
            .flush()
            .map_err(|e| anyhow::anyhow!("OpenOCD flush error: {e}"))?;

        // 逐行读取直到遇到提示符 "> "
        let mut reader = BufReader::new(stream.by_ref());
        let mut response = String::new();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    if line == "> " || line.starts_with("> ") {
                        break;
                    }
                    response.push_str(&line);
                }
                Err(e) => {
                    anyhow::bail!("OpenOCD command timeout (cmd: {cmd}): {e}");
                }
            }
        }

        Ok(response)
    }

    /// 清理子进程：先优雅 exit，超时后 force kill。
    fn cleanup_process(&mut self) {
        // 先尝试发送 exit 命令优雅关闭
        if self.telnet.is_some() {
            let _ = self.tcl_command("exit");
        }
        self.telnet = None;

        // 回收子进程
        if let Some(mut child) = self.process.take() {
            // 轮询等待子进程退出（最多 5 秒）
            for _ in 0..25 {
                match child.try_wait() {
                    Ok(Some(_)) => break, // 正常退出
                    Ok(None) => std::thread::sleep(Duration::from_millis(200)),
                    Err(_) => break,
                }
            }
            // 超时或出错，强制 kill
            let _ = child.kill();
            let _ = child.wait();
        }

        // 清空所有状态（支持 re-attach）
        self.target_halted = false;
        self.bp_map.clear();
        self.wp_map.clear();
        self.next_bp_id = 0;
        self.next_wp_id = 0;
    }
}

impl Drop for OpenOcdBackend {
    fn drop(&mut self) {
        // 析构时确保子进程被清理，不留僵尸
        if self.process.is_some() {
            self.cleanup_process();
        }
    }
}

impl DebugProbe for OpenOcdBackend {
    fn attach(&mut self, _chip: &ChipConfig) -> anyhow::Result<()> {
        // 解析配置文件路径
        let resolved_cfg = match &self.cfg_path {
            Some(path) => path.clone(),
            None => ".debugger/openocd.cfg".to_string(),
        };

        // 校验文件存在
        if !Path::new(&resolved_cfg).exists() {
            anyhow::bail!(
                "OpenOCD cfg file not found: {resolved_cfg} \
                 (use --openocd-cfg or place .debugger/openocd.cfg)"
            );
        }

        // 启动 OpenOCD 子进程
        self.spawn_openocd(&resolved_cfg)?;

        // 等待 TCP 端口就绪
        let stream = self.wait_for_telnet()?;
        self.telnet = Some(stream);

        Ok(())
    }

    fn detach(&mut self) -> anyhow::Result<()> {
        self.cleanup_process();
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.telnet.is_some()
    }

    fn try_recover(&mut self) -> anyhow::Result<()> {
        anyhow::bail!("P2: OpenOCD recovery not yet implemented")
    }

    fn flash(&mut self, elf: &Path, opts: &FlashOpts) -> anyhow::Result<()> {
        // 构建 program 命令：
        //   program <elf> [verify]   — 烧录 + 可选校验
        // 注：reset/exit 不由 flash() 处理，交给后续 resume/detach
        let verify_flag = if opts.verify { "verify" } else { "" };
        let elf_path = elf
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("cannot resolve ELF path: {e}"))?
            .to_string_lossy()
            .to_string();

        let cmd = format!("program {elf_path} {verify_flag}");
        let response = self.tcl_command(&cmd)?;

        // 检查成功关键词
        let response_lower = response.to_lowercase();
        if response_lower.contains("error")
            || response_lower.contains("failed")
            || response_lower.contains("invalid")
        {
            anyhow::bail!("OpenOCD flash failed: {response}");
        }

        Ok(())
    }

    fn halt(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        let response = self.tcl_command("halt")?;
        let response_lower = response.to_lowercase();
        if response_lower.contains("error") || response_lower.contains("failed") {
            anyhow::bail!("OpenOCD halt failed: {response}");
        }
        self.target_halted = true;
        Ok(())
    }

    fn resume(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        // Q2=A: halted 态发 resume，非 halted 态发 reset run（flash 场景兼容）
        let cmd = if self.target_halted {
            "resume"
        } else {
            "reset run"
        };
        let response = self.tcl_command(cmd)?;
        let response_lower = response.to_lowercase();
        if response_lower.contains("error") || response_lower.contains("failed") {
            anyhow::bail!("OpenOCD resume ({cmd}) failed: {response}");
        }
        self.target_halted = false;
        Ok(())
    }

    fn step(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        let response = self.tcl_command("step")?;
        let response_lower = response.to_lowercase();
        if response_lower.contains("error") || response_lower.contains("failed") {
            anyhow::bail!("OpenOCD step failed: {response}");
        }
        self.target_halted = true;
        Ok(())
    }

    fn core_count(&self) -> usize {
        1 // Standalone flash 默认单核
    }

    fn active_core(&self) -> usize {
        0
    }

    fn set_breakpoint(&mut self, addr: u32, _core: Option<usize>) -> anyhow::Result<BpId> {
        let addr = addr as u64;
        // 先分配 ID，再操作硬件（先改 self、再借子对象模式）
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        self.bp_map.insert(addr, id);

        let result = self.tcl_command(&format!("bp 0x{addr:x} 4 hw"));
        match result {
            Ok(response) => {
                let response_lower = response.to_lowercase();
                if response_lower.contains("error") || response_lower.contains("failed") {
                    // 回滚
                    self.bp_map.remove(&addr);
                    self.next_bp_id -= 1;
                    anyhow::bail!("OpenOCD set_breakpoint at 0x{addr:x} failed: {response}");
                }
                Ok(id)
            }
            Err(e) => {
                // 回滚
                self.bp_map.remove(&addr);
                self.next_bp_id -= 1;
                Err(anyhow::anyhow!(
                    "OpenOCD set_breakpoint at 0x{addr:x} failed: {e}"
                ))
            }
        }
    }

    fn clear_breakpoint(&mut self, id: BpId) -> anyhow::Result<()> {
        // 从 bp_map 反查地址
        let addr = self
            .bp_map
            .iter()
            .find(|&(_, bid)| *bid == id)
            .map(|(&a, _)| a)
            .ok_or_else(|| anyhow::anyhow!("breakpoint #{id} not found"))?;

        let response = self.tcl_command(&format!("rbp 0x{addr:x}"))?;
        let response_lower = response.to_lowercase();
        if response_lower.contains("error") || response_lower.contains("failed") {
            anyhow::bail!("OpenOCD clear_breakpoint #{id} at 0x{addr:x} failed: {response}");
        }
        self.bp_map.retain(|&a, _| a != addr);
        Ok(())
    }

    fn set_watchpoint(&mut self, addr: u32, len: u32, kind: WatchKind) -> anyhow::Result<WpId> {
        let addr = addr as u64;
        let flag = match kind {
            WatchKind::Read => "r",
            WatchKind::Write => "w",
            WatchKind::ReadWrite => "a",
        };
        let id = self.next_wp_id;
        self.next_wp_id += 1;
        self.wp_map.insert(addr, id);

        let cmd = format!("wp 0x{addr:x} {len} {flag}");
        match self.tcl_command(&cmd) {
            Ok(response) => {
                let l = response.to_lowercase();
                if l.contains("error") || l.contains("failed") {
                    self.wp_map.remove(&addr);
                    self.next_wp_id -= 1;
                    anyhow::bail!("OpenOCD set_watchpoint at 0x{addr:x} failed: {response}");
                }
                Ok(id)
            }
            Err(e) => {
                self.wp_map.remove(&addr);
                self.next_wp_id -= 1;
                Err(anyhow::anyhow!(
                    "OpenOCD set_watchpoint at 0x{addr:x} failed: {e}"
                ))
            }
        }
    }

    fn clear_watchpoint(&mut self, id: WpId) -> anyhow::Result<()> {
        let addr = self
            .wp_map
            .iter()
            .find(|&(_, wid)| *wid == id)
            .map(|(&a, _)| a)
            .ok_or_else(|| anyhow::anyhow!("watchpoint #{id} not found"))?;

        let response = self.tcl_command(&format!("rwp 0x{addr:x}"))?;
        let l = response.to_lowercase();
        if l.contains("error") || l.contains("failed") {
            anyhow::bail!("OpenOCD clear_watchpoint #{id} at 0x{addr:x} failed: {response}");
        }
        self.wp_map.retain(|&a, _| a != addr);
        Ok(())
    }

    fn read_mem(&mut self, addr: u32, len: u32, _core: Option<usize>) -> anyhow::Result<Vec<u8>> {
        // OpenOCD 以 word (32-bit) 为单位读内存
        let count = (len + 3) / 4;
        let cmd = format!("read_memory 0x{addr:x} 32 {count}");
        let response = self.tcl_command(&cmd)?;

        // 解析 Tcl 返回的 word 列表:
        // 格式1: 每个 word 单独一行
        // 格式2: {word1 word2 ...} 花括号括起的列表
        // 去掉花括号
        let cleaned = response
            .trim()
            .trim_start_matches('{')
            .trim_end_matches('}');

        let mut result = Vec::with_capacity(len as usize);
        for token in cleaned.split_whitespace() {
            if token.is_empty() {
                continue;
            }
            // 可能带 0x 前缀
            let val = if let Some(hex) = token
                .strip_prefix("0x")
                .or_else(|| token.strip_prefix("0X"))
            {
                u32::from_str_radix(hex, 16)
            } else {
                token.parse::<u32>()
            };
            match val {
                Ok(word) => {
                    result.extend_from_slice(&word.to_le_bytes());
                }
                Err(_) => {
                    continue;
                }
            }
            if result.len() >= len as usize {
                break;
            }
        }
        result.truncate(len as usize);
        Ok(result)
    }

    fn write_mem(&mut self, addr: u32, data: &[u8], _core: Option<usize>) -> anyhow::Result<()> {
        // 将 data 按 4 字节对齐并拆分为 u32 列表
        let words: Vec<String> = data
            .chunks(4)
            .map(|chunk| {
                let mut buf = [0u8; 4];
                buf[..chunk.len()].copy_from_slice(chunk);
                let word = u32::from_le_bytes(buf);
                format!("0x{word:x}")
            })
            .collect();

        let cmd = format!("write_memory 0x{addr:x} 32 {{{}}}", words.join(" "));
        let response = self.tcl_command(&cmd)?;
        let response_lower = response.to_lowercase();
        if response_lower.contains("error") || response_lower.contains("failed") {
            anyhow::bail!("OpenOCD write_mem at 0x{addr:x} failed: {response}");
        }
        Ok(())
    }

    fn read_regs(&mut self, _core: Option<usize>) -> anyhow::Result<HashMap<String, u64>> {
        let response = self.tcl_command("reg")?;
        let mut map = HashMap::new();

        for line in response.lines() {
            let trimmed = line.trim();
            // 匹配格式: "(n) name (/bits): 0xVALUE" 或 "(n) name (/bits): 0xVALUE (dirty)"
            if !trimmed.starts_with('(') {
                continue;
            }
            // 去掉开头的括号编号
            let after_paren = match trimmed.find(')') {
                Some(idx) => &trimmed[idx + 1..],
                None => continue,
            };
            let parts: Vec<&str> = after_paren.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }
            let name_part = parts[0].trim();
            let value_part = parts[1].trim();

            // 提取寄存器名（去掉括号中的比特数部分）
            let name = match name_part.find(" (") {
                Some(idx) => name_part[..idx].trim(),
                None => name_part,
            };
            if name.is_empty() {
                continue;
            }

            // 提取值（可能含 " (dirty)" 后缀）
            let val_str = value_part.split_whitespace().next().unwrap_or("");
            let val = if let Some(hex) = val_str
                .strip_prefix("0x")
                .or_else(|| val_str.strip_prefix("0X"))
            {
                u64::from_str_radix(hex, 16).ok()
            } else {
                val_str.parse::<u64>().ok()
            };

            if let Some(v) = val {
                map.insert(name.to_string(), v);
            }
        }

        Ok(map)
    }

    fn is_halted(&mut self, _core: Option<usize>) -> bool {
        self.target_halted
    }

    fn poll_halted(&mut self, _core: Option<usize>) -> bool {
        if self.target_halted {
            return true;
        }
        // 用 1ms 短超时轮询目标是否刚刚 halt
        match self.tcl_command("wait_halt 1") {
            Ok(resp) => {
                let l = resp.to_lowercase();
                if !l.contains("error") && !l.contains("failed") && !l.contains("timeout") {
                    self.target_halted = true;
                    return true;
                }
                false
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ChipConfig, OpenOcdBackend};
    use crate::probe::DebugProbe;

    /// 验证后端创建时的初始状态。
    #[test]
    fn test_openocd_creation() {
        let mut backend = OpenOcdBackend::new(None);
        assert!(!backend.is_connected());
        assert_eq!(backend.core_count(), 1);
        assert_eq!(backend.active_core(), 0);
        assert!(!backend.is_halted(None));
    }

    /// 配置文件不存在时返回 Err。
    #[test]
    fn test_openocd_attach_no_cfg() {
        let mut backend = OpenOcdBackend::new(Some("nonexistent.cfg".into()));
        let chip = ChipConfig {
            name: "test".into(),
            architecture: "cortex-m4".into(),
            flash_base: 0,
            flash_size: 0x100000,
            ram_base: 0x20000000,
            ram_size: 0x20000,
        };
        let err = backend.attach(&chip).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("cfg file not found"),
            "Expected cfg not found, got: {msg}"
        );
    }

    /// 未连接时 halt 应返回 Err。
    #[test]
    fn test_openocd_halt_without_attach() {
        let mut backend = OpenOcdBackend::new(None);
        let err = backend.halt(None).unwrap_err();
        assert!(
            err.to_string().contains("not connected"),
            "Expected 'not connected', got: {err}"
        );
    }

    /// 未连接时 step 应返回 Err。
    #[test]
    fn test_openocd_step_without_attach() {
        let mut backend = OpenOcdBackend::new(None);
        let err = backend.step(None).unwrap_err();
        assert!(
            err.to_string().contains("not connected"),
            "Expected 'not connected', got: {err}"
        );
    }

    /// 未连接时 resume 应返回 Err。
    #[test]
    fn test_openocd_resume_without_attach() {
        let mut backend = OpenOcdBackend::new(None);
        let err = backend.resume(None).unwrap_err();
        assert!(
            err.to_string().contains("not connected"),
            "Expected 'not connected', got: {err}"
        );
    }

    /// 未连接时 set_breakpoint 应返回 Err。
    #[test]
    fn test_openocd_set_breakpoint_without_attach() {
        let mut backend = OpenOcdBackend::new(None);
        let err = backend.set_breakpoint(0x08000100, None).unwrap_err();
        assert!(
            err.to_string().contains("not connected"),
            "Expected 'not connected', got: {err}"
        );
    }

    /// clear_breakpoint 不存在的 ID 应返回 Err。
    #[test]
    fn test_openocd_clear_breakpoint_not_found() {
        let mut backend = OpenOcdBackend::new(None);
        let err = backend.clear_breakpoint(42).unwrap_err();
        assert!(
            err.to_string().contains("breakpoint #42 not found"),
            "Expected 'breakpoint not found', got: {err}"
        );
    }

    /// 未连接时 read_mem 应返回 Err。
    #[test]
    fn test_openocd_read_mem_without_attach() {
        let mut backend = OpenOcdBackend::new(None);
        let err = backend.read_mem(0x20000000, 16, None).unwrap_err();
        assert!(
            err.to_string().contains("not connected"),
            "Expected 'not connected', got: {err}"
        );
    }

    /// 未连接时 read_regs 应返回 Err。
    #[test]
    fn test_openocd_read_regs_without_attach() {
        let mut backend = OpenOcdBackend::new(None);
        let err = backend.read_regs(None).unwrap_err();
        assert!(
            err.to_string().contains("not connected"),
            "Expected 'not connected', got: {err}"
        );
    }

    /// detach 在未连接状态下应是安全的（幂等）。
    #[test]
    fn test_openocd_detach_is_idempotent() {
        let mut backend = OpenOcdBackend::new(None);
        assert!(backend.detach().is_ok());
        assert!(backend.detach().is_ok());
    }

    /// set_watchpoint 未连接时返回 Err。
    #[test]
    fn test_openocd_watchpoint_without_attach() {
        let mut backend = OpenOcdBackend::new(None);
        use crate::probe::WatchKind;
        assert!(
            backend
                .set_watchpoint(0x20000000, 4, WatchKind::ReadWrite)
                .is_err()
        );
        assert!(backend.clear_watchpoint(0).is_err());
    }
}
