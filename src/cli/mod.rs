/// CLI 定义 — 使用 clap derive 宏。
///
/// 设计文档 §4.1 定义了四个顶层子命令:
///   init → 生成 .debugger/chip.toml
///   flash → 烧录 ELF
///   clean → 清理缓存
///   debug → 启动调试会话 (REPL 或 JSON-Lines)
pub mod clean;
pub mod debug;
pub mod flash;
pub mod init;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// mcu-bridge — 面向 AI Agent 的嵌入式调试中间件
#[derive(Parser)]
#[command(name = "mcu-bridge", version, about = "面向 AI Agent 的嵌入式调试中间件", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 初始化芯片配置，生成 .debugger/chip.toml
    Init {
        /// 芯片型号，如 STM32F407VG
        #[arg(long)]
        chip: String,
        /// 调试器类型: stlink-v2 | jlink | cmsis-dap | ftdi
        #[arg(long)]
        debugger: Option<String>,
        /// 调试接口: swd | jtag
        #[arg(long)]
        interface: Option<String>,
    },

    /// 烧录 ELF 固件到目标芯片
    Flash {
        /// ELF 文件路径
        #[arg(long)]
        elf: PathBuf,
        /// 烧录后执行回读校验
        #[arg(long, default_value_t = true)]
        verify: bool,
        /// 芯片型号（默认从 ELF 中检测）
        #[arg(long)]
        chip: Option<String>,
        /// 烧录完成后自动复位运行（默认 halt）
        #[arg(long)]
        run: bool,
        /// 强制指定后端: probe-rs | openocd
        #[arg(long)]
        backend: Option<String>,
        /// OpenOCD 配置文件路径（仅 --backend openocd 时生效）
        #[arg(long)]
        openocd_cfg: Option<String>,
    },

    /// 清理缓存目录 (~/.mcu_bridge/)
    Clean {
        /// 清理所有项目的缓存（而非仅当前项目）
        #[arg(long)]
        all: bool,
        /// 清理 N 天前的缓存，如 7d / 30d
        #[arg(long)]
        older_than: Option<String>,
    },

    /// 启动调试会话（Human REPL 或 Agent JSON-Lines）
    Debug {
        /// ELF 文件路径
        #[arg(long)]
        elf: PathBuf,
        /// 配置文件路径（默认自动查找 .debugger/chip.toml）
        #[arg(long)]
        config: Option<PathBuf>,
        /// Agent JSON-Lines 模式（默认 Human REPL）
        #[arg(long)]
        json: bool,
        /// 跳过烧录步骤
        #[arg(long)]
        no_flash: bool,
        /// 烧录后校验
        #[arg(long)]
        verify: bool,
        /// 强制指定后端: probe-rs | openocd
        #[arg(long)]
        backend: Option<String>,
        /// 启用 Flash 断点
        #[arg(long)]
        enable_flash_bp: bool,
        /// 启动后立即设断点（可重复，逗号分隔）
        #[arg(long = "break", value_delimiter = ',')]
        break_at: Vec<String>,
        /// 启动后立即设 watch（variable,size 格式，可重复，逗号分隔）
        #[arg(long = "watch", value_delimiter = ',')]
        watch_targets: Vec<String>,
        /// 启动后立即 continue
        #[arg(long)]
        continue_: bool,
        /// 无条件在 reset vector 处 halt
        #[arg(long)]
        halt_on_start: bool,
        /// 覆盖采样间隔 (ms)
        #[arg(long)]
        sampling_interval: Option<u64>,
        /// 覆盖串口端口
        #[arg(long)]
        serial_port: Option<String>,
    },
}
