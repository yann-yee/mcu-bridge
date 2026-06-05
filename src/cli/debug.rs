//! debug 子命令 — 启动调试会话。
//!
//! 设计文档 §4.2：双模式界面
//!   Human REPL — 交互式 `> ` 提示符，彩色输出
//!   Agent JSON-Lines — stdin→JSON，stdout→JSON，`--json` 模式
//!
//! 启动时先 `attach` 探针 → 进入 HALTED 态 → 等待用户/Agent 命令。
//!
//! ⚠ 部分 CLI 参数是 P2/P3 预留（config/verify/enable_flash_bp）。

#![allow(dead_code)]

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, mpsc};

use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::buffer::serial::SerialMonitor;
use crate::buffer::{DebugBuffer, LogBuffer, Sampler};
use crate::cli::init;
use crate::cli::json_session::JsonSession;
use crate::config::{ChipConfig, FlashOpts};
use crate::dwarf::DwarfResolver;
use crate::log::detect_log_backend;
use crate::probe::DebugProbe;
use crate::probe::openocd::OpenOcdBackend;
use crate::probe::probe_rs::ProbeRsBackend;
use crate::session::{Session, SessionState};

/// debug 子命令参数
pub struct DebugArgs {
    pub elf: PathBuf,
    pub chip: Option<String>,
    pub config: Option<PathBuf>,
    pub json: bool,
    pub no_flash: bool,
    pub verify: bool,
    pub backend: Option<String>,
    pub enable_flash_bp: bool,
    pub break_at: Vec<String>,
    pub watch_targets: Vec<String>,
    pub continue_: bool,
    pub halt_on_start: bool,
    pub sampling_interval: Option<u64>,
    pub serial_port: Option<String>,
    pub openocd_cfg: Option<String>,
}

/// REPL 命令枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// 暂停目标
    Halt,
    /// 全速运行
    Resume,
    /// 单步执行
    Step,
    /// 设硬件断点
    Break { addr: u32 },
    /// 显示寄存器
    Regs,
    /// 读取内存
    Mem { addr: u32, len: u32 },
    /// 显示会话状态
    Status,
    /// 显示帮助
    Help,
    /// 退出会话
    Quit,
    /// 添加 watch target
    Watch {
        addr: u32,
        size: u32,
        label: Option<String>,
    },
    /// 查询采样历史
    Buffer {
        since: Option<u64>,
        watch_id: Option<usize>,
    },
    /// 查询日志历史（serial read）
    Serial {
        since: Option<u64>,
        channel: Option<String>,
    },
    /// 查询符号信息（DWARF）
    Info {
        subcmd: InfoSubcmd,
    },
}

/// info 子命令
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfoSubcmd {
    Functions,
    Variables,
    Symbol(String),
}

impl fmt::Display for InfoSubcmd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Functions => write!(f, "functions"),
            Self::Variables => write!(f, "variables"),
            Self::Symbol(name) => write!(f, "symbol {name}"),
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Halt => write!(f, "halt"),
            Self::Resume => write!(f, "resume"),
            Self::Step => write!(f, "step"),
            Self::Break { addr } => write!(f, "break 0x{addr:08x}"),
            Self::Regs => write!(f, "regs"),
            Self::Mem { addr, len } => write!(f, "mem 0x{addr:08x} {len}"),
            Self::Status => write!(f, "status"),
            Self::Help => write!(f, "help"),
            Self::Quit => write!(f, "quit"),
            Self::Watch { addr, size, label } => {
                if let Some(l) = label {
                    write!(f, "watch 0x{addr:08x}:{size}:{l}")
                } else {
                    write!(f, "watch 0x{addr:08x}:{size}")
                }
            }
            Self::Buffer { since, watch_id } => {
                write!(f, "buffer")?;
                if let Some(s) = since {
                    write!(f, " {s}")?;
                }
                if let Some(w) = watch_id {
                    write!(f, " {w}")?;
                }
                Ok(())
            }
            Self::Serial { since, channel } => {
                write!(f, "serial")?;
                if let Some(s) = since {
                    write!(f, " {s}")?;
                }
                if let Some(ch) = channel {
                    write!(f, " {ch}")?;
                }
                Ok(())
            }
            Self::Info { subcmd } => write!(f, "info {subcmd}"),
        }
    }
}

impl Command {
    /// 从用户输入的字符串解析命令。
    ///
    /// 支持 `0x` 前缀十六进制地址和纯十进制数。
    /// 如果提供 `dwarf` 解析器，函数名/变量名可代替地址。
    /// 返回 `Err` 时包含人类可读的错误消息。
    pub fn parse(input: &str, dwarf: Option<&DwarfResolver>) -> Result<Self, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("empty input".into());
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd = parts[0].to_lowercase();

        match cmd.as_str() {
            "halt" => {
                if parts.len() != 1 {
                    return Err("usage: halt".into());
                }
                Ok(Self::Halt)
            }
            "resume" | "go" => {
                if parts.len() != 1 {
                    return Err("usage: resume".into());
                }
                Ok(Self::Resume)
            }
            "step" | "s" => {
                if parts.len() != 1 {
                    return Err("usage: step".into());
                }
                Ok(Self::Step)
            }
            "break" | "b" => {
                if parts.len() < 2 {
                    return Err("usage: break <addr|funcname>".into());
                }
                let addr = if let Ok(a) = parse_u32(parts[1]) {
                    a
                } else if let Some(resolver) = dwarf {
                    resolver.function_addr(parts[1]).ok_or_else(|| {
                        format!(
                            "cannot resolve '{}' as hex address or function name",
                            parts[1]
                        )
                    })?
                } else {
                    return Err(format!(
                        "invalid address: '{}'. Use hex (0x...) or decimal.",
                        parts[1]
                    ));
                };
                Ok(Self::Break { addr })
            }
            "regs" | "registers" => {
                if parts.len() != 1 {
                    return Err("usage: regs".into());
                }
                Ok(Self::Regs)
            }
            "mem" | "memory" | "mdw" => {
                if parts.len() < 3 {
                    return Err("usage: mem <addr> <len>".into());
                }
                let addr = parse_u32(parts[1]).map_err(|_| {
                    format!(
                        "invalid address: '{}'. Use hex (0x...) or decimal.",
                        parts[1]
                    )
                })?;
                let len = parts[2]
                    .parse::<u32>()
                    .map_err(|_| format!("invalid length: '{}'. Use decimal.", parts[2]))?;
                Ok(Self::Mem { addr, len })
            }
            "status" | "st" => {
                if parts.len() != 1 {
                    return Err("usage: status".into());
                }
                Ok(Self::Status)
            }
            "help" | "h" | "?" => Ok(Self::Help),
            "quit" | "exit" | "q" => Ok(Self::Quit),
            "watch" | "w" => {
                if parts.len() < 2 {
                    return Err("usage: watch <addr|varname>[:size[:label]]".into());
                }
                let (addr, size, label) = resolve_watch_spec(parts[1], dwarf)?;
                Ok(Self::Watch { addr, size, label })
            }
            "buffer" | "buff" => {
                // positional: buffer [since] [watch_id]
                let since = if parts.len() > 1 {
                    Some(
                        parts[1]
                            .parse::<u64>()
                            .map_err(|_| format!("invalid sn: '{}'. Use decimal.", parts[1]))?,
                    )
                } else {
                    None
                };
                let watch_id =
                    if parts.len() > 2 {
                        Some(parts[2].parse::<usize>().map_err(|_| {
                            format!("invalid watch id: '{}'. Use decimal.", parts[2])
                        })?)
                    } else {
                        None
                    };
                Ok(Self::Buffer { since, watch_id })
            }
            "serial" => {
                // serial [since] [channel]
                let since = if parts.len() > 1 {
                    Some(
                        parts[1]
                            .parse::<u64>()
                            .map_err(|_| format!("invalid sn: '{}'. Use decimal.", parts[1]))?,
                    )
                } else {
                    None
                };
                let channel = if parts.len() > 2 {
                    Some(parts[2].to_string())
                } else {
                    None
                };
                Ok(Self::Serial { since, channel })
            }
            "info" => {
                let subcmd = match parts.get(1).map(|s| *s) {
                    Some("functions") | Some("funcs") => InfoSubcmd::Functions,
                    Some("variables") | Some("vars") => InfoSubcmd::Variables,
                    Some(name) => InfoSubcmd::Symbol(name.to_string()),
                    None => {
                        return Err("usage: info <functions|variables|symbol <name>>".into());
                    }
                };
                Ok(Self::Info { subcmd })
            }
            _ => Err(format!(
                "unknown command '{}'. Type 'help' for available commands.",
                parts[0]
            )),
        }
    }

    /// 该命令在哪些会话状态下合法。
    ///
    /// 返回 `None` 表示在所有状态下均合法。
    pub fn valid_states(&self) -> Option<&[SessionState]> {
        match self {
            Self::Halt => Some(&[SessionState::Running]),
            Self::Resume | Self::Step | Self::Break { .. } | Self::Regs | Self::Mem { .. } => {
                Some(&[SessionState::Halted])
            }
            Self::Status | Self::Help | Self::Quit | Self::Buffer { .. } | Self::Serial { .. }
            | Self::Info { .. } => {
                None
            }
            Self::Watch { .. } => Some(&[SessionState::Halted]),
        }
    }
}

/// 解析十六进制或十进制地址字符串为 u32。
fn parse_u32(s: &str) -> Result<u32, std::num::ParseIntError> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16)
    } else {
        s.parse::<u32>()
    }
}

/// 解析 watch 规格，支持地址格式和 DWARF 变量名格式。
///
/// 格式:
/// - `0x20000000:4:label` — 地址:大小:标签
/// - `adc_val` — 变量名（自动推导大小）
/// - `adc_val:2` — 变量名:覆盖大小
fn resolve_watch_spec(spec: &str, dwarf: Option<&DwarfResolver>) -> Result<(u32, u32, Option<String>), String> {
    let colons: Vec<&str> = spec.split(':').collect();

    // 尝试将第一段解析为十六进制地址
    if let Ok(addr) = parse_u32(colons[0]) {
        // 地址格式：addr:size[:label]
        let size = if colons.len() > 1 {
            colons[1]
                .parse::<u32>()
                .map_err(|_| format!("invalid size: '{}'. Use decimal.", colons[1]))?
        } else {
            return Err("watch spec requires size when using hex address, e.g. 0x20000000:4".into());
        };
        if !matches!(size, 1 | 2 | 4 | 8) {
            return Err(format!("watch size must be 1, 2, 4, or 8, got {size}"));
        }
        let label = colons.get(2).filter(|s| !s.is_empty()).map(|s| s.to_string());
        return Ok((addr, size, label));
    }

    // 地址失败 → 尝试 DWARF 变量名
    if let Some(resolver) = dwarf {
        let var = resolver
            .variable_info(colons[0])
            .ok_or_else(|| format!("cannot resolve '{}' as address or variable name", colons[0]))?;
        let size = if colons.len() > 1 {
            let s = colons[1]
                .parse::<u32>()
                .map_err(|_| format!("invalid size: '{}'. Use decimal.", colons[1]))?;
            if !matches!(s, 1 | 2 | 4 | 8) {
                return Err(format!("watch size must be 1, 2, 4, or 8, got {s}"));
            }
            s
        } else {
            var.size
        };
        let label = colons
            .get(2)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| Some(colons[0].to_string()));
        return Ok((var.addr, size, label));
    }

    Err(format!(
        "invalid address: '{}'. Use hex (0x...) or decimal.",
        colons[0]
    ))
}

/// 交互式调试 REPL
pub struct DebugRepl {
    /// 调试会话
    session: Session,
    /// rustyline 行编辑器
    rl: DefaultEditor,
    /// 共享调试缓冲区
    buffer: Arc<RwLock<DebugBuffer>>,
    /// 采样线程句柄
    sampler_thread: Option<std::thread::JoinHandle<()>>,
    /// 采样停止信号
    sampler_stop: Option<Arc<AtomicBool>>,
    /// 采样间隔（ms）
    sampling_interval: u64,
    /// DWARF 符号解析器
    dwarf: Option<DwarfResolver>,
}

impl DebugRepl {
    /// 创建 REPL 实例。
    pub fn new(
        session: Session,
        sampling_interval: u64,
        buffer_capacity: usize,
        dwarf: Option<DwarfResolver>,
    ) -> Self {
        let rl = DefaultEditor::new().unwrap_or_else(|_| DefaultEditor::new().unwrap());
        Self {
            session,
            rl,
            buffer: Arc::new(RwLock::new(DebugBuffer::new(buffer_capacity))),
            sampler_thread: None,
            sampler_stop: None,
            sampling_interval,
            dwarf,
        }
    }

    /// 进入主交互循环，直至用户 quit 或出现致命错误。
    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            match self.read_command() {
                Some(Command::Quit) => break,
                Some(cmd) => {
                    // 状态守卫
                    if let Some(states) = cmd.valid_states()
                        && !states.contains(&self.session.state)
                    {
                        println!(
                            "[ERROR] command '{cmd}' not valid in {:?} state",
                            self.session.state
                        );
                        continue;
                    }
                    // 执行
                    if let Err(e) = self.execute(cmd) {
                        println!("[ERROR] {e}");
                    }
                }
                None => continue,
            }
        }
        // 确保采样线程在 quit 前停止
        self.stop_sampler();
        self.session.detach()?;
        println!("[OK] debug session ended");
        Ok(())
    }

    /// 读取一行输入，尝试解析为 Command。
    fn read_command(&mut self) -> Option<Command> {
        match self.rl.readline("(mcu) > ") {
            Ok(line) => {
                self.rl.add_history_entry(&line).ok();
                match Command::parse(&line, self.dwarf.as_ref()) {
                    Ok(cmd) => Some(cmd),
                    Err(e) => {
                        if !line.trim().is_empty() {
                            println!("[ERROR] {e}");
                        }
                        None
                    }
                }
            }
            Err(ReadlineError::Interrupted | ReadlineError::Eof) => Some(Command::Quit),
            Err(err) => {
                println!("[ERROR] readline error: {err}");
                None
            }
        }
    }

    /// 检查命令在当前状态下是否合法，若合法则执行。
    fn execute(&mut self, cmd: Command) -> anyhow::Result<()> {
        match cmd {
            Command::Halt => self.cmd_halt(),
            Command::Resume => self.cmd_resume(),
            Command::Step => self.cmd_step(),
            Command::Break { addr } => self.cmd_break(addr),
            Command::Regs => self.cmd_regs(),
            Command::Mem { addr, len } => self.cmd_mem(addr, len),
            Command::Status => {
                self.cmd_status();
                Ok(())
            }
            Command::Help => {
                self.print_help();
                Ok(())
            }
            Command::Quit => Ok(()), // handled by run()
            Command::Watch { addr, size, label } => self.cmd_watch(addr, size, label),
            Command::Buffer { since, watch_id } => self.cmd_buffer(since, watch_id),
            Command::Serial { since, channel } => self.cmd_serial(since, channel),
            Command::Info { subcmd } => self.cmd_info(subcmd),
        }
    }

    // ── 命令实现 ──

    /// 暂停目标
    fn cmd_halt(&mut self) -> anyhow::Result<()> {
        // 先停止采样线程
        self.stop_sampler();
        self.session
            .backend
            .lock()
            .expect("backend lock")
            .halt(None)?;
        self.session.state = SessionState::Halted;
        println!("[OK] target halted");
        Ok(())
    }

    /// 全速运行 — 自动启动采样线程
    fn cmd_resume(&mut self) -> anyhow::Result<()> {
        if self.sampler_thread.is_some() {
            anyhow::bail!("sampler thread already running, halt first");
        }
        self.session
            .backend
            .lock()
            .expect("backend lock")
            .resume(None)?;
        self.session.state = SessionState::Running;

        // 如果有 watch target，启动采样线程
        let watch_count = self.buffer.read().unwrap().targets.len();
        if watch_count > 0 {
            let backend = self.session.shared_backend();
            let buffer = self.buffer.clone();
            let mut sampler = Sampler::new(backend, buffer, self.sampling_interval, 0);
            let stop_flag = sampler.stop_flag();
            self.sampler_stop = Some(stop_flag);
            self.sampler_thread = Some(std::thread::spawn(move || {
                sampler.run();
            }));
            println!(
                "[OK] target running | sampling {watch_count} target(s) @ {}ms",
                self.sampling_interval
            );
        } else {
            println!("[OK] target running (no watch targets, sampling not started)");
        }
        Ok(())
    }

    /// 停止采样线程（最多等待 2 秒，超时则分离线程不阻塞主线程）。
    fn stop_sampler(&mut self) {
        if let Some(stop) = self.sampler_stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(handle) = self.sampler_thread.take() {
            // 等最多 2s 让采样线程看到 stop_flag 并退出
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while std::time::Instant::now() < deadline {
                if handle.is_finished() {
                    let _ = handle.join();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            // 超时：分离线程（采样卡在 read_mem 硬件超时中），不阻塞主线程
            // JoinHandle drop → 线程被分离，它会在 http 下回归
        }
    }

    /// 添加 watch target
    fn cmd_watch(&mut self, addr: u32, size: u32, label: Option<String>) -> anyhow::Result<()> {
        let id = self
            .buffer
            .write()
            .unwrap()
            .add_target(addr, size, label.clone());
        let label = label.unwrap_or_else(|| format!("0x{addr:08x}"));
        println!("[#{}] watching {label} at 0x{addr:08x} (size={size})", id);
        self.session.watch_count = self.buffer.read().unwrap().targets.len();
        Ok(())
    }

    /// 查询采样历史
    fn cmd_buffer(&self, since: Option<u64>, watch_id: Option<usize>) -> anyhow::Result<()> {
        let samples = self.buffer.read().unwrap().get_samples(watch_id, since);
        if samples.is_empty() || samples.iter().all(|v| v.is_empty()) {
            println!("[OK] no samples");
            return Ok(());
        }
        // 获取标签列表（在 guard 内拷贝）
        let labels: Vec<String> = {
            let guard = self.buffer.read().unwrap();
            guard.targets.iter().map(|t| t.label.clone()).collect()
        };
        for (i, buf) in samples.iter().enumerate() {
            let label = labels.get(i).map(|s| s.as_str()).unwrap_or("?");
            println!("--- {label} ({} samples) ---", buf.len());
            for sample in buf {
                println!(
                    "#{sn} t={tick} val=0x{val:x} bp={bp}",
                    sn = sample.sn,
                    tick = sample.tick_us,
                    val = sample.val,
                    bp = if sample.bp_flag { "Y" } else { "N" }
                );
            }
        }
        Ok(())
    }

    /// 查询日志历史
    fn cmd_serial(&self, _since: Option<u64>, _channel: Option<String>) -> anyhow::Result<()> {
        println!("[NOTE] serial log viewing is only available in JSON-Lines mode");
        Ok(())
    }

    /// 查询符号信息（DWARF）。
    fn cmd_info(&self, subcmd: InfoSubcmd) -> anyhow::Result<()> {
        let dwarf = match self.dwarf.as_ref() {
            Some(d) => d,
            None => {
                println!("[ERROR] no DWARF info available (load an ELF with --elf)");
                return Ok(());
            }
        };
        match subcmd {
            InfoSubcmd::Functions => {
                let funcs = dwarf.list_functions();
                if funcs.is_empty() {
                    println!("[INFO] no functions found in DWARF");
                    return Ok(());
                }
                println!("Functions ({}):", funcs.len());
                for f in funcs {
                    println!("  {:<30} 0x{:08x}..0x{:08x} ({} bytes)", f.name, f.low_addr, f.high_addr, f.high_addr - f.low_addr);
                }
            }
            InfoSubcmd::Variables => {
                let vars = dwarf.list_variables();
                if vars.is_empty() {
                    println!("[INFO] no global variables found in DWARF");
                    return Ok(());
                }
                println!("Global variables ({}):", vars.len());
                for v in vars {
                    println!("  {:<30} 0x{:08x} size={}", v.name, v.addr, v.size);
                }
            }
            InfoSubcmd::Symbol(name) => {
                // 尝试作为函数查询
                if let Some(addr) = dwarf.function_addr(&name) {
                    println!("function '{}' @ 0x{:08x}", name, addr);
                    return Ok(());
                }
                // 尝试作为变量查询
                if let Some(var) = dwarf.variable_info(&name) {
                    println!(
                        "variable '{}' @ 0x{:08x} size={} type={:?}",
                        name, var.addr, var.size, var.type_name
                    );
                    return Ok(());
                }
                println!("[ERROR] '{}' not found in DWARF symbols", name);
            }
        }
        Ok(())
    }

    /// 单步执行
    fn cmd_step(&mut self) -> anyhow::Result<()> {
        self.session
            .backend
            .lock()
            .expect("backend lock")
            .step(None)?;
        // 读取 PC 显示当前位置
        if let Ok(regs) = self
            .session
            .backend
            .lock()
            .expect("backend lock")
            .read_regs(None)
        {
            if let Some(pc_val) = regs.get("pc").or_else(|| regs.get("PC")) {
                let pc = *pc_val as u32;
                self.session.pc = Some(pc);
                println!("[OK] stepped to 0x{pc:08x}");
            } else {
                println!("[OK] step");
            }
        } else {
            println!("[OK] step");
        }
        self.session.state = SessionState::Halted;
        Ok(())
    }

    /// 设硬件断点
    fn cmd_break(&mut self, addr: u32) -> anyhow::Result<()> {
        let id = self
            .session
            .backend
            .lock()
            .unwrap()
            .set_breakpoint(addr, None)?;
        self.session.bp_count += 1;
        println!("[#{}] breakpoint at 0x{addr:08x}", id);
        Ok(())
    }

    /// 显示寄存器
    fn cmd_regs(&mut self) -> anyhow::Result<()> {
        let regs = self
            .session
            .backend
            .lock()
            .expect("backend lock")
            .read_regs(None)?;
        // 收集并排序 key
        let mut keys: Vec<&String> = regs.keys().collect();
        keys.sort();
        for key in keys {
            if let Some(val) = regs.get(key) {
                println!("{key}\t0x{val:08x}");
            }
        }
        // 更新 PC
        if let Some(pc_val) = regs.get("pc").or_else(|| regs.get("PC")) {
            self.session.pc = Some(*pc_val as u32);
        }
        Ok(())
    }

    /// 读取内存
    fn cmd_mem(&mut self, addr: u32, len: u32) -> anyhow::Result<()> {
        let data = self
            .session
            .backend
            .lock()
            .unwrap()
            .read_mem(addr, len, None)?;
        // 十六进制 dump，每行 16 字节
        for (i, chunk) in data.chunks(16).enumerate() {
            let line_addr = addr + (i as u32 * 16);
            // 十六进制部分
            let hex: String = chunk
                .iter()
                .enumerate()
                .map(|(j, b)| {
                    if j == 8 {
                        format!(" {:02x}", b) // 中间加空格
                    } else {
                        format!("{:02x}", b)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ");
            // ASCII 部分
            let ascii: String = chunk
                .iter()
                .map(|b| {
                    if b.is_ascii_graphic() || *b == b' ' {
                        *b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            // 补齐 hex 到 16 字节宽度（约 48 字符）
            let hex_padded = format!("{:<48}", hex);
            println!("0x{line_addr:08x}  {hex_padded}  {ascii}");
        }
        Ok(())
    }

    /// 显示会话状态
    fn cmd_status(&self) {
        let pc_str = self
            .session
            .pc
            .map(|p| format!("0x{p:08x}"))
            .unwrap_or_else(|| "?".into());
        println!(
            "state={:?}  chip={}  bp={}  pc={}  cores={}",
            self.session.state,
            self.session.chip_name,
            self.session.bp_count,
            pc_str,
            self.session.core_count
        );
    }

    /// 显示帮助
    fn print_help(&self) {
        println!("Available commands:");
        println!("  halt              Pause target execution");
        println!("  resume, go        Resume target execution (starts sampler)");
        println!("  step, s           Single-step (halted)");
        println!("  break <addr|funcname>, b  Set hardware breakpoint (halted)");
        println!("  regs, registers   Show core registers (halted)");
        println!("  mem <addr> <len>  Read memory (halted)");
        println!("  watch <a|varname>[:<s>[:l]] Add watch target (halted)");
        println!("  buffer [since] [watch_id] Show sampling history");
        println!("  serial [since] [channel]  Show log history (JSON-Lines mode)");
        println!("  info functions    List functions from DWARF");
        println!("  info variables    List global variables from DWARF");
        println!("  info symbol <n>   Look up a symbol in DWARF");
        println!("  buffer [since] [watch_id] Show sampling history");
        println!("  serial [since] [channel]  Show log history (JSON-Lines mode)");
        println!("  status, st        Show session status");
        println!("  help, h, ?        Show this help");
        println!("  quit, exit, q     Exit debug session");
    }
}

/// 解析芯片配置（CLI 参数优先，否则尝试 .debugger/chip.toml）。
fn resolve_chip_for_debug(chip_arg: &Option<String>) -> anyhow::Result<ChipConfig> {
    match chip_arg {
        Some(name) => {
            let mut chip = init::get_chip_template(name)?;
            chip.name = name.clone();
            Ok(chip)
        }
        None => {
            // 尝试读取 .debugger/chip.toml
            let path = Path::new(".debugger/chip.toml");
            if path.exists() {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
                let app: crate::config::AppConfig = toml::from_str(&content)
                    .map_err(|e| anyhow::anyhow!("config parse error: {e}"))?;
                Ok(app.chip)
            } else {
                anyhow::bail!(
                    "no --chip argument and no .debugger/chip.toml found. \
                     Use --chip <NAME> or run 'mcu-bridge init --chip <NAME>' first."
                )
            }
        }
    }
}

/// 解析芯片配置并构造 FlashOpts。
fn resolve_chip_and_flash_opts(
    chip_arg: &Option<String>,
) -> anyhow::Result<(ChipConfig, FlashOpts)> {
    let chip = resolve_chip_for_debug(chip_arg)?;
    let opts = FlashOpts {
        base: chip.flash_base,
        size: chip.flash_size,
        sections: vec![crate::config::FlashSection {
            name: "app".into(),
            addr: chip.flash_base,
            len: chip.flash_size,
        }],
        verify: true,
    };
    Ok((chip, opts))
}

/// 创建调试后端（根据 `--backend` CLI 参数）。
fn create_debug_backend(
    backend_arg: &Option<String>,
    openocd_cfg: &Option<String>,
) -> anyhow::Result<Box<dyn DebugProbe>> {
    let backend_type = backend_arg.as_deref().unwrap_or("probe-rs");
    match backend_type.to_ascii_lowercase().as_str() {
        "probe-rs" => Ok(Box::new(ProbeRsBackend::new())),
        "openocd" => {
            // 优先级: --openocd-cfg CLI > .debugger/openocd.cfg 兜底
            let cfg_path = openocd_cfg.clone().or_else(|| {
                let default_path = Path::new(".debugger/openocd.cfg");
                if default_path.exists() {
                    Some(default_path.to_string_lossy().to_string())
                } else {
                    None
                }
            });
            match cfg_path {
                Some(path) => Ok(Box::new(OpenOcdBackend::new(Some(path)))),
                None => anyhow::bail!(
                    "OpenOCD backend requires a config file. \
                     Use --openocd-cfg <PATH> or create .debugger/openocd.cfg"
                ),
            }
        }
        _ => anyhow::bail!("unknown backend '{backend_type}'. Supported: probe-rs, openocd"),
    }
}

/// 处理 debug 子命令
pub fn handle(args: &DebugArgs) -> anyhow::Result<()> {
    // 校验 ELF 文件存在
    if !args.elf.exists() {
        anyhow::bail!("ELF file not found: {}", args.elf.display());
    }

    // 解析芯片配置 + 烧录参数
    let (chip, flash_opts) = resolve_chip_and_flash_opts(&args.chip)?;

    // 创建后端
    let backend = create_debug_backend(&args.backend, &args.openocd_cfg)?;

    // 连接并创建会话
    let mut session = Session::attach(&chip, backend)?;
    println!(
        "[OK] attached to {}, {} core(s)",
        chip.name, session.core_count
    );

    // 烧录固件（除非 --no-flash）
    if !args.no_flash {
        println!("[INFO] flashing ELF...");
        session
            .backend
            .lock()
            .unwrap()
            .flash(&args.elf, &flash_opts)?;
        println!("[OK] flash complete");
    }

    // 设置启动断点（--break-at）
    for addr_str in &args.break_at {
        let addr = parse_u32(addr_str)
            .map_err(|_| anyhow::anyhow!("invalid breakpoint address: '{addr_str}'"))?;
        let id = session
            .backend
            .lock()
            .expect("backend lock")
            .set_breakpoint(addr, None)?;
        println!("[#{}] breakpoint at 0x{addr:08x}", id);
    }

    // halt-on-start 优先；仅当未指定 halt-on-start 且指定了 continue 时才 resume
    if !args.halt_on_start && args.continue_ {
        session.backend.lock().expect("backend lock").resume(None)?;
        session.state = SessionState::Running;
        println!("[OK] target running");
    }

    // 采样与观测配置
    let sampling_interval = args.sampling_interval.unwrap_or(10);
    let buffer_capacity = 128;

    // 日志通道检测（可选 — 不阻止会话继续）
    let log_buffer = Arc::new(RwLock::new(LogBuffer::new(4096)));
    let (log_event_tx, log_event_rx) = mpsc::channel();

    // 路由到对应界面
    if args.json {
        let shared_backend = session.shared_backend();
        let mut js = JsonSession::new(
            session,
            sampling_interval,
            buffer_capacity,
            log_buffer.clone(),
            Some(log_event_rx),
        );

        // JSON 模式：尝试检测日志后端并启动 SerialMonitor
        if let Some(ch) = detect_log_backend(shared_backend, args.serial_port.clone(), 0) {
            let mut monitor = SerialMonitor::new(ch, log_buffer, log_event_tx, 100);
            monitor.start();
            // SerialMonitor 线程在 JsonSession::run() 期间运行
            // 当 JsonSession 退出时，monitor 会被 drop 自动停止
            js.run()?;
            // 确保 SerialMonitor 在会话结束前停止
            let _ = monitor.stop();
        } else {
            log::info!("no log backend available, running without serial monitor");
            js.run()?;
        }
    } else {
        // 加载 DWARF 符号信息（如果 ELF 可用）
        let dwarf = if args.elf.as_os_str().is_empty() {
            None
        } else {
            match DwarfResolver::from_elf(&args.elf) {
                Ok(d) => {
                    log::info!("DWARF loaded: {} functions, {} variables", d.function_count(), d.variable_count());
                    Some(d)
                }
                Err(e) => {
                    log::warn!("failed to load DWARF from '{}': {e}", args.elf.display());
                    None
                }
            }
        };

        let mut repl = DebugRepl::new(session, sampling_interval, buffer_capacity, dwarf);

        // 处理 --watch 参数（启动时添加观测目标）
        for watch_spec in &args.watch_targets {
            let (addr, size, label) = DebugBuffer::parse_watch_spec(watch_spec)
                .map_err(|e| anyhow::anyhow!("--watch parse error: {e}"))?;
            repl.cmd_watch(addr, size, label)?;
        }

        // 如果指定了 --continue_ 且有 watch target，resume 会自动启动采样
        // 如果指定了 --watch 但没有 --continue_，用户手动 resume 时启动采样

        repl.run()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用 parse 包装，默认无 DWARF 解析器。
    fn p(input: &str) -> Result<Command, String> {
        Command::parse(input, None)
    }

    // ── 命令解析测试 ──

    #[test]
    fn test_parse_halt() {
        assert_eq!(p("halt").unwrap(), Command::Halt);
    }

    #[test]
    fn test_parse_resume() {
        assert_eq!(p("resume").unwrap(), Command::Resume);
        assert_eq!(p("go").unwrap(), Command::Resume);
    }

    #[test]
    fn test_parse_step() {
        assert_eq!(p("step").unwrap(), Command::Step);
        assert_eq!(p("s").unwrap(), Command::Step);
    }

    #[test]
    fn test_parse_break() {
        assert_eq!(
            p("break 0x08000100").unwrap(),
            Command::Break { addr: 0x08000100 }
        );
    }

    #[test]
    fn test_parse_break_decimal() {
        // 0x08000100 = 134217984
        assert_eq!(
            p("break 134217984").unwrap(),
            Command::Break { addr: 0x08000100 }
        );
    }

    #[test]
    fn test_parse_break_no_addr() {
        assert!(p("break").is_err());
    }

    #[test]
    fn test_parse_break_bad_addr() {
        assert!(p("break abc").is_err());
    }

    #[test]
    fn test_parse_break_short() {
        assert_eq!(
            p("b 0x20000000").unwrap(),
            Command::Break { addr: 0x20000000 }
        );
    }

    #[test]
    fn test_parse_regs() {
        assert_eq!(p("regs").unwrap(), Command::Regs);
        assert_eq!(p("registers").unwrap(), Command::Regs);
    }

    #[test]
    fn test_parse_mem() {
        assert_eq!(
            p("mem 0x20000000 16").unwrap(),
            Command::Mem {
                addr: 0x20000000,
                len: 16
            }
        );
    }

    #[test]
    fn test_parse_mem_missing_len() {
        assert!(p("mem 0x20000000").is_err());
    }

    #[test]
    fn test_parse_status() {
        assert_eq!(p("status").unwrap(), Command::Status);
        assert_eq!(p("st").unwrap(), Command::Status);
    }

    #[test]
    fn test_parse_help() {
        assert_eq!(p("help").unwrap(), Command::Help);
        assert_eq!(p("h").unwrap(), Command::Help);
        assert_eq!(p("?").unwrap(), Command::Help);
    }

    #[test]
    fn test_parse_quit() {
        assert_eq!(p("quit").unwrap(), Command::Quit);
        assert_eq!(p("exit").unwrap(), Command::Quit);
        assert_eq!(p("q").unwrap(), Command::Quit);
    }

    #[test]
    fn test_parse_unknown() {
        assert!(p("xyz").is_err());
    }

    #[test]
    fn test_parse_whitespace() {
        assert!(p("").is_err());
        assert!(p("  ").is_err());
    }

    #[test]
    fn test_parse_trailing_whitespace() {
        assert_eq!(p("halt  ").unwrap(), Command::Halt);
    }

    // ── 状态守卫测试 ──

    #[test]
    fn test_halt_valid_in_running() {
        let states = Command::Halt.valid_states().unwrap();
        assert!(states.contains(&SessionState::Running));
    }

    #[test]
    fn test_halt_invalid_in_halted() {
        let states = Command::Halt.valid_states().unwrap();
        assert!(!states.contains(&SessionState::Halted));
    }

    #[test]
    fn test_resume_valid_in_halted() {
        let states = Command::Resume.valid_states().unwrap();
        assert!(states.contains(&SessionState::Halted));
    }

    #[test]
    fn test_resume_invalid_in_running() {
        let states = Command::Resume.valid_states().unwrap();
        assert!(!states.contains(&SessionState::Running));
    }

    #[test]
    fn test_step_valid_in_halted() {
        let states = Command::Step.valid_states().unwrap();
        assert!(states.contains(&SessionState::Halted));
    }

    #[test]
    fn test_step_invalid_in_running() {
        let states = Command::Step.valid_states().unwrap();
        assert!(!states.contains(&SessionState::Running));
    }

    #[test]
    fn test_break_valid_in_halted() {
        let states = Command::Break { addr: 0 }.valid_states().unwrap();
        assert!(states.contains(&SessionState::Halted));
    }

    #[test]
    fn test_status_all_states() {
        assert!(Command::Status.valid_states().is_none());
    }

    #[test]
    fn test_help_quit_all_states() {
        assert!(Command::Help.valid_states().is_none());
        assert!(Command::Quit.valid_states().is_none());
    }

    // ── 启动错误路径测试 ──

    #[test]
    fn test_debug_elf_not_found() {
        use std::path::PathBuf;

        let args = DebugArgs {
            elf: PathBuf::from("nonexistent.elf"),
            chip: Some("STM32F407VG".into()),
            config: None,
            json: false,
            no_flash: false,
            verify: true,
            backend: None,
            enable_flash_bp: false,
            break_at: vec![],
            watch_targets: vec![],
            continue_: false,
            halt_on_start: false,
            sampling_interval: None,
            serial_port: None,
            openocd_cfg: None,
        };
        let err = handle(&args).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("ELF file not found"),
            "Expected 'ELF file not found', got: {msg}"
        );
    }

    #[test]
    fn test_debug_unknown_chip() {
        use std::path::PathBuf;

        let args = DebugArgs {
            elf: PathBuf::from("Cargo.toml"),
            chip: Some("INVALID".into()),
            config: None,
            json: false,
            no_flash: false,
            verify: true,
            backend: None,
            enable_flash_bp: false,
            break_at: vec![],
            watch_targets: vec![],
            continue_: false,
            halt_on_start: false,
            sampling_interval: None,
            serial_port: None,
            openocd_cfg: None,
        };
        let err = handle(&args).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown chip"),
            "Expected 'unknown chip', got: {msg}"
        );
    }

    #[test]
    fn test_debug_no_chip_no_config() {
        use std::path::PathBuf;

        let args = DebugArgs {
            elf: PathBuf::from("Cargo.toml"),
            chip: None,
            config: None,
            json: false,
            no_flash: false,
            verify: true,
            backend: None,
            enable_flash_bp: false,
            break_at: vec![],
            watch_targets: vec![],
            continue_: false,
            halt_on_start: false,
            sampling_interval: None,
            serial_port: None,
            openocd_cfg: None,
        };
        let err = handle(&args).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("chip") || msg.contains("config") || msg.contains("not found"),
            "Expected chip/config error, got: {msg}"
        );
    }

    // ── create_debug_backend 测试 ──

    #[test]
    fn test_create_backend_probe_rs_default() {
        assert!(create_debug_backend(&None, &None).is_ok());
    }

    #[test]
    fn test_create_backend_unknown() {
        let result = create_debug_backend(&Some("invalid".into()), &None);
        assert!(result.is_err());
        // 验证错误信息 (通过 to_string 获取)
        let err_msg = match result {
            Err(e) => e.to_string(),
            Ok(_) => unreachable!(),
        };
        assert!(
            err_msg.contains("unknown backend"),
            "Expected 'unknown backend', got: {err_msg}"
        );
    }
}
