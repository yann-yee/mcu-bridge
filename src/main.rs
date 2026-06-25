//! Binary entry point for `mcu-bridge`.

use clap::Parser;

use mcu_bridge::cli::{self, Cli, Commands};

fn main() -> anyhow::Result<()> {
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
            no_verify,
            chip,
            run,
            backend,
            openocd_cfg,
            json,
        } => {
            cli::flash::handle(&cli::flash::FlashArgs {
                elf,
                no_verify,
                chip,
                run,
                backend,
                openocd_cfg,
                json,
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
            no_verify,
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
                no_verify,
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
        Commands::Doctor {
            chip,
            backend,
            openocd_cfg,
            json,
        } => {
            cli::doctor::handle(&cli::doctor::DoctorArgs {
                chip,
                backend,
                openocd_cfg,
                json,
            })?;
        }
    }

    Ok(())
}
