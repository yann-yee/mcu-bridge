//! mcu-bridge — 面向 AI Agent 的嵌入式调试中间件。
//!
//! 入口点：解析 CLI 子命令，分发到对应处理函数。

mod buffer;
mod cli;
mod config;
mod dwarf;
mod error;
mod log;
mod probe;
mod session;

use clap::Parser;

use crate::cli::{Cli, Commands};

fn main() -> anyhow::Result<()> {
    // 初始化日志（默认 WARN 级别，可通过 RUST_LOG 环境变量覆盖）
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .format_timestamp_millis()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            chip,
            debugger,
            interface,
        } => {
            cli::init::handle(&cli::init::InitArgs {
                chip,
                debugger,
                interface,
            })?;
        }
        Commands::Flash {
            elf,
            verify,
            chip,
            run,
            backend,
            openocd_cfg,
        } => {
            cli::flash::handle(&cli::flash::FlashArgs {
                elf,
                verify,
                chip,
                run,
                backend,
                openocd_cfg,
            })?;
        }
        Commands::Clean { all, older_than } => {
            cli::clean::handle(&cli::clean::CleanArgs { all, older_than })?;
        }
        Commands::Debug {
            elf,
            config,
            json,
            no_flash,
            verify,
            backend,
            enable_flash_bp,
            break_at,
            watch_targets,
            continue_,
            halt_on_start,
            sampling_interval,
            serial_port,
            chip,
            openocd_cfg,
        } => {
            cli::debug::handle(&cli::debug::DebugArgs {
                elf,
                chip,
                config,
                json,
                no_flash,
                verify,
                backend,
                enable_flash_bp,
                break_at,
                watch_targets,
                continue_,
                halt_on_start,
                sampling_interval,
                serial_port,
                openocd_cfg,
            })?;
        }
    }

    Ok(())
}
