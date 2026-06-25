//! Non-mutating target diagnostics.

use crate::backend::{
    load_default_app_config, load_optional_default_app_config, resolve_backend_mode,
    resolve_backend_order, resolve_openocd_cfg,
};
use crate::cli::init;
use crate::config::{AppConfig, ChipConfig};
use crate::operations::{DoctorRunOptions, run_doctor};

/// Arguments for the `doctor` subcommand.
pub struct DoctorArgs {
    pub chip: Option<String>,
    pub backend: Option<String>,
    pub openocd_cfg: Option<String>,
    pub json: bool,
}

/// Handle `mcu-bridge doctor`.
pub fn handle(args: &DoctorArgs) -> anyhow::Result<()> {
    let config_attempt = load_default_app_config();
    let config = config_attempt.as_ref().ok();
    let chip = resolve_chip(args.chip.as_deref(), config)?;
    let backend_mode = resolve_backend_mode(args.backend.as_deref(), config);
    let backend_order = resolve_backend_order(&backend_mode, config)?;
    let openocd_cfg = resolve_openocd_cfg(args.openocd_cfg.as_deref(), config);

    let report = run_doctor(&DoctorRunOptions {
        chip,
        backend_order,
        openocd_cfg,
        config_ok: config_attempt.is_ok(),
        config_error: config_attempt.err().map(|err| err.to_string()),
    });

    if args.json {
        println!("{}", serde_json::to_string(&report)?);
    } else {
        render_human_report(&report);
    }

    Ok(())
}

fn resolve_chip(chip_arg: Option<&str>, config: Option<&AppConfig>) -> anyhow::Result<ChipConfig> {
    if let Some(name) = chip_arg {
        let mut chip = init::get_chip_template(name)?;
        chip.name = name.to_string();
        return Ok(chip);
    }

    config
        .map(|cfg| cfg.chip.clone())
        .or_else(|| load_optional_default_app_config().map(|cfg| cfg.chip))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no --chip argument and no .debugger/chip.toml found. Use --chip <NAME> or run 'mcu-bridge init --chip <NAME>' first."
            )
        })
}

fn render_human_report(report: &crate::operations::DoctorReport) {
    println!("chip: {}", report.chip);
    println!("config_ok: {}", report.config_ok);
    if let Some(config_error) = &report.config_error {
        println!("config_error: {config_error}");
    }
    if let Some(backend) = &report.recommended_backend {
        println!("recommended_backend: {backend}");
    }
    for check in &report.backend_checks {
        if let Some(error) = &check.error {
            println!(
                "{}: error ({}) {}",
                check.backend, error.code, error.message
            );
        } else {
            println!("{}: ok", check.backend);
        }
    }
    if let Some(state) = &report.target_state {
        println!("halted: {}", state.halted);
    }
    if let Some(registers) = &report.registers {
        println!(
            "pc: {} sp: {} xpsr: {}",
            format_hex(registers.pc),
            format_hex(registers.sp),
            format_hex(registers.xpsr)
        );
    }
    if let Some(fault) = &report.fault_summary {
        println!(
            "faults: hfsr={} cfsr={} mmfar={} bfar={}",
            format_hex(Some(fault.hfsr)),
            format_hex(Some(fault.cfsr)),
            format_hex(Some(fault.mmfar)),
            format_hex(Some(fault.bfar))
        );
    }
}

fn format_hex(value: Option<u32>) -> String {
    value
        .map(|value| format!("0x{value:08x}"))
        .unwrap_or_else(|| "-".into())
}
