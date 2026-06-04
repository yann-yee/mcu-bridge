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
                    Ok(Some(_)) => return, // 正常退出
                    Ok(None) => std::thread::sleep(Duration::from_millis(200)),
                    Err(_) => break,
                }
            }
            // 超时或出错，强制 kill
            let _ = child.kill();
            let _ = child.wait();
        }
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
        anyhow::bail!("P2: OpenOCD halt not implemented for standalone flash")
    }

    fn resume(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        // Standalone flash 场景：flash() 已用 program 命令完成烧录，
        // 用 reset 命令让芯片复位运行（不经过 detach/exit）
        let response = self.tcl_command("reset")?;
        let response_lower = response.to_lowercase();
        if response_lower.contains("error") || response_lower.contains("failed") {
            anyhow::bail!("OpenOCD resume failed: {response}");
        }
        Ok(())
    }

    fn step(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        anyhow::bail!("P2: OpenOCD step not implemented")
    }

    fn core_count(&self) -> usize {
        1 // Standalone flash 默认单核
    }

    fn active_core(&self) -> usize {
        0
    }

    fn set_breakpoint(&mut self, _addr: u32, _core: Option<usize>) -> anyhow::Result<BpId> {
        anyhow::bail!("P2: OpenOCD set_breakpoint not implemented")
    }

    fn clear_breakpoint(&mut self, _id: BpId) -> anyhow::Result<()> {
        anyhow::bail!("P2: OpenOCD clear_breakpoint not implemented")
    }

    fn set_watchpoint(&mut self, _addr: u32, _len: u32, _kind: WatchKind) -> anyhow::Result<WpId> {
        anyhow::bail!("P2: OpenOCD watchpoint not implemented")
    }

    fn clear_watchpoint(&mut self, _id: WpId) -> anyhow::Result<()> {
        anyhow::bail!("P2: OpenOCD clear_watchpoint not implemented")
    }

    fn read_mem(&mut self, _addr: u32, _len: u32, _core: Option<usize>) -> anyhow::Result<Vec<u8>> {
        anyhow::bail!("P2: OpenOCD read_mem not implemented")
    }

    fn write_mem(&mut self, _addr: u32, _data: &[u8], _core: Option<usize>) -> anyhow::Result<()> {
        anyhow::bail!("P2: OpenOCD write_mem not implemented")
    }

    fn read_regs(&mut self, _core: Option<usize>) -> anyhow::Result<HashMap<String, u64>> {
        anyhow::bail!("P2: OpenOCD read_regs not implemented")
    }

    fn is_halted(&self, _core: Option<usize>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{ChipConfig, OpenOcdBackend};
    use crate::probe::DebugProbe;

    /// 验证后端创建时的初始状态。
    #[test]
    fn test_openocd_creation() {
        let backend = OpenOcdBackend::new(None);
        assert!(!backend.is_connected());
        assert_eq!(backend.core_count(), 1);
    }

    /// 配置文件不存在时返回 Err(E_PARAM)。
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
}
