/// CLI definitions.
pub mod clean;
pub mod debug;
pub mod doctor;
pub mod flash;
pub mod init;
pub mod json_session;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Top-level CLI.
#[derive(Parser)]
#[command(
    name = "mcu-bridge",
    version,
    about = "Host-side MCU debug bridge for agents",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Top-level subcommands.
#[derive(Subcommand)]
pub enum Commands {
    /// Generate `.debugger/chip.toml`.
    Init {
        #[arg(long)]
        chip: String,
        #[arg(long)]
        debugger: Option<String>,
        #[arg(long)]
        interface: Option<String>,
    },

    /// Flash an ELF image to the target.
    Flash {
        #[arg(long)]
        elf: PathBuf,
        #[arg(long)]
        no_verify: bool,
        #[arg(long)]
        chip: Option<String>,
        #[arg(long)]
        run: bool,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        openocd_cfg: Option<String>,
        #[arg(long)]
        json: bool,
    },

    /// Clean the local cache directory.
    Clean {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        older_than: Option<String>,
    },

    /// Start an interactive or JSON debug session.
    Debug {
        #[arg(long)]
        elf: PathBuf,
        #[arg(long)]
        chip: Option<String>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        no_flash: bool,
        #[arg(long)]
        no_verify: bool,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        enable_flash_bp: bool,
        #[arg(long = "break", value_delimiter = ',')]
        break_at: Vec<String>,
        #[arg(long = "watch", value_delimiter = ',')]
        watch_targets: Vec<String>,
        #[arg(long)]
        continue_: bool,
        #[arg(long)]
        halt_on_start: bool,
        #[arg(long)]
        sampling_interval: Option<u64>,
        #[arg(long)]
        serial_port: Option<String>,
        #[arg(long)]
        openocd_cfg: Option<String>,
    },

    /// Run non-mutating target diagnostics.
    Doctor {
        #[arg(long)]
        chip: Option<String>,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        openocd_cfg: Option<String>,
        #[arg(long)]
        json: bool,
    },
}
