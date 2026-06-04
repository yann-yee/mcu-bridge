//! debug 子命令 — 启动调试会话。
//!
//! 设计文档 §4.2：双模式界面
//!   Human REPL — 交互式 `> ` 提示符，彩色输出
//!   Agent JSON-Lines — stdin→JSON，stdout→JSON，`--json` 模式
//!
//! 启动时先 `attach` 探针 → 进入 HALTED 态 → 等待用户/Agent 命令。

use std::fmt;
use std::path::{Path, PathBuf};

use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::cli::init;
use crate::config::ChipConfig;
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
        }
    }
}

impl Command {
    /// 从用户输入的字符串解析命令。
    ///
    /// 支持 `0x` 前缀十六进制地址和纯十进制数。
    /// 返回 `Err` 时包含人类可读的错误消息。
    pub fn parse(input: &str) -> Result<Self, String> {
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
                    return Err("usage: break <addr>".into());
                }
                let addr = parse_u32(parts[1]).map_err(|_| {
                    format!(
                        "invalid address: '{}'. Use hex (0x...) or decimal.",
                        parts[1]
                    )
                })?;
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
            Self::Status | Self::Help | Self::Quit => None,
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

/// 交互式调试 REPL
pub struct DebugRepl {
    /// 调试会话
    session: Session,
    /// rustyline 行编辑器
    rl: DefaultEditor,
}

impl DebugRepl {
    /// 创建 REPL 实例。
    pub fn new(session: Session) -> Self {
        let rl = DefaultEditor::new().unwrap_or_else(|_| {
            // 如果无法创建编辑器，使用无历史回退
            DefaultEditor::new().unwrap()
        });
        Self { session, rl }
    }

    /// 进入主交互循环，直至用户 quit 或出现致命错误。
    pub fn run(&mut self) -> anyhow::Result<()> {
        loop {
            match self.read_command() {
                Some(Command::Quit) => break,
                Some(cmd) => {
                    // 状态守卫
                    if let Some(states) = cmd.valid_states() {
                        if !states.contains(&self.session.state) {
                            println!(
                                "[ERROR] command '{cmd}' not valid in {:?} state",
                                self.session.state
                            );
                            continue;
                        }
                    }
                    // 执行
                    if let Err(e) = self.execute(cmd) {
                        println!("[ERROR] {e}");
                    }
                }
                None => continue,
            }
        }
        self.session.detach()?;
        println!("[OK] debug session ended");
        Ok(())
    }

    /// 读取一行输入，尝试解析为 Command。
    fn read_command(&mut self) -> Option<Command> {
        match self.rl.readline("(mcu) > ") {
            Ok(line) => {
                self.rl.add_history_entry(&line).ok();
                match Command::parse(&line) {
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
        }
    }

    // ── 命令实现 ──

    /// 暂停目标
    fn cmd_halt(&mut self) -> anyhow::Result<()> {
        self.session.backend.halt(None)?;
        self.session.state = SessionState::Halted;
        println!("[OK] target halted");
        Ok(())
    }

    /// 全速运行
    fn cmd_resume(&mut self) -> anyhow::Result<()> {
        self.session.backend.resume(None)?;
        self.session.state = SessionState::Running;
        println!("[OK] target running");
        Ok(())
    }

    /// 单步执行
    fn cmd_step(&mut self) -> anyhow::Result<()> {
        self.session.backend.step(None)?;
        // 读取 PC 显示当前位置
        if let Ok(regs) = self.session.backend.read_regs(None) {
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
        let id = self.session.backend.set_breakpoint(addr, None)?;
        self.session.bp_count += 1;
        println!("[#{}] breakpoint at 0x{addr:08x}", id);
        Ok(())
    }

    /// 显示寄存器
    fn cmd_regs(&mut self) -> anyhow::Result<()> {
        let regs = self.session.backend.read_regs(None)?;
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
        let data = self.session.backend.read_mem(addr, len, None)?;
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
        println!("  resume, go        Resume target execution");
        println!("  step, s           Single-step (halted)");
        println!("  break <addr>, b   Set hardware breakpoint (halted)");
        println!("  regs, registers   Show core registers (halted)");
        println!("  mem <addr> <len>  Read memory (halted)");
        println!("  status, st        Show session status");
        println!("  help, h, ?        Show this help");
        println!("  quit, exit, q     Exit debug session");
    }
}

/// 解析芯片配置（CLI 参数优先，否则尝试 .debugger/chip.toml）。
fn resolve_chip_for_debug<'a>(chip_arg: &'a Option<String>) -> anyhow::Result<ChipConfig> {
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

/// 处理 debug 子命令
pub fn handle(args: &DebugArgs) -> anyhow::Result<()> {
    // 校验 ELF 文件存在
    if !args.elf.exists() {
        anyhow::bail!("ELF file not found: {}", args.elf.display());
    }

    // 解析芯片配置
    let chip = resolve_chip_for_debug(&args.chip)?;

    // TODO: Round 2 — 处理以下参数
    //   --json / --break-at / --watch / --continue_ / --halt-on-start
    //   --no-flash / --verify / --backend / --enable-flash-bp
    //   --sampling-interval / --serial-port / --config

    // 连接并创建会话
    let session = Session::attach(&chip)?;
    println!(
        "[OK] attached to {}, {} core(s)",
        chip.name, session.core_count
    );

    // 进入 REPL 循环
    let mut repl = DebugRepl::new(session);
    repl.run()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 命令解析测试 ──

    #[test]
    fn test_parse_halt() {
        assert_eq!(Command::parse("halt").unwrap(), Command::Halt);
    }

    #[test]
    fn test_parse_resume() {
        assert_eq!(Command::parse("resume").unwrap(), Command::Resume);
        assert_eq!(Command::parse("go").unwrap(), Command::Resume);
    }

    #[test]
    fn test_parse_step() {
        assert_eq!(Command::parse("step").unwrap(), Command::Step);
        assert_eq!(Command::parse("s").unwrap(), Command::Step);
    }

    #[test]
    fn test_parse_break() {
        assert_eq!(
            Command::parse("break 0x08000100").unwrap(),
            Command::Break { addr: 0x08000100 }
        );
    }

    #[test]
    fn test_parse_break_decimal() {
        // 0x08000100 = 134217984
        assert_eq!(
            Command::parse("break 134217984").unwrap(),
            Command::Break { addr: 0x08000100 }
        );
    }

    #[test]
    fn test_parse_break_no_addr() {
        assert!(Command::parse("break").is_err());
    }

    #[test]
    fn test_parse_break_bad_addr() {
        assert!(Command::parse("break abc").is_err());
    }

    #[test]
    fn test_parse_break_short() {
        assert_eq!(
            Command::parse("b 0x20000000").unwrap(),
            Command::Break { addr: 0x20000000 }
        );
    }

    #[test]
    fn test_parse_regs() {
        assert_eq!(Command::parse("regs").unwrap(), Command::Regs);
        assert_eq!(Command::parse("registers").unwrap(), Command::Regs);
    }

    #[test]
    fn test_parse_mem() {
        assert_eq!(
            Command::parse("mem 0x20000000 16").unwrap(),
            Command::Mem {
                addr: 0x20000000,
                len: 16
            }
        );
    }

    #[test]
    fn test_parse_mem_missing_len() {
        assert!(Command::parse("mem 0x20000000").is_err());
    }

    #[test]
    fn test_parse_status() {
        assert_eq!(Command::parse("status").unwrap(), Command::Status);
        assert_eq!(Command::parse("st").unwrap(), Command::Status);
    }

    #[test]
    fn test_parse_help() {
        assert_eq!(Command::parse("help").unwrap(), Command::Help);
        assert_eq!(Command::parse("h").unwrap(), Command::Help);
        assert_eq!(Command::parse("?").unwrap(), Command::Help);
    }

    #[test]
    fn test_parse_quit() {
        assert_eq!(Command::parse("quit").unwrap(), Command::Quit);
        assert_eq!(Command::parse("exit").unwrap(), Command::Quit);
        assert_eq!(Command::parse("q").unwrap(), Command::Quit);
    }

    #[test]
    fn test_parse_unknown() {
        assert!(Command::parse("xyz").is_err());
    }

    #[test]
    fn test_parse_whitespace() {
        assert!(Command::parse("").is_err());
        assert!(Command::parse("  ").is_err());
    }

    #[test]
    fn test_parse_trailing_whitespace() {
        assert_eq!(Command::parse("halt  ").unwrap(), Command::Halt);
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
        };
        let err = handle(&args).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("chip") || msg.contains("config") || msg.contains("not found"),
            "Expected chip/config error, got: {msg}"
        );
    }
}
