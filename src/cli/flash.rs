//! Flash command implementation.

use std::path::PathBuf;

use crate::backend::{
    load_optional_default_app_config, resolve_backend_mode, resolve_backend_order,
    resolve_openocd_cfg,
};
use crate::cli::init;
use crate::config::{AppConfig, ChipConfig, FlashOpts, FlashSection};
use crate::operations::{FlashReport, FlashRunOptions, OperationStage, run_flash};

/// Arguments for the `flash` subcommand.
pub struct FlashArgs {
    pub elf: PathBuf,
    pub no_verify: bool,
    pub chip: Option<String>,
    pub run: bool,
    pub backend: Option<String>,
    pub openocd_cfg: Option<String>,
    pub json: bool,
}

/// Handle `mcu-bridge flash`.
pub fn handle(args: &FlashArgs) -> anyhow::Result<()> {
    if !args.elf.exists() {
        anyhow::bail!("ELF file not found: {}", args.elf.display());
    }

    let config = load_optional_default_app_config();
    let (chip, flash) = resolve_chip_and_flash(args, config.as_ref())?;
    let backend_mode = resolve_backend_mode(args.backend.as_deref(), config.as_ref());
    let backend_order = resolve_backend_order(&backend_mode, config.as_ref())?;
    let openocd_cfg = resolve_openocd_cfg(args.openocd_cfg.as_deref(), config.as_ref());

    let report = run_flash(&FlashRunOptions {
        elf: args.elf.clone(),
        chip,
        flash,
        run: args.run,
        backend_order,
        openocd_cfg,
    });

    if args.json {
        println!("{}", serde_json::to_string(&report)?);
    } else {
        render_human_report(&report);
    }

    if report.success {
        Ok(())
    } else {
        anyhow::bail!(
            "{}",
            report
                .error
                .as_ref()
                .map(|err| err.message.as_str())
                .unwrap_or("flash failed")
        )
    }
}

fn resolve_chip_and_flash(
    args: &FlashArgs,
    config: Option<&AppConfig>,
) -> anyhow::Result<(ChipConfig, FlashOpts)> {
    if let Some(name) = args.chip.as_deref() {
        let mut chip = init::get_chip_template(name)?;
        chip.name = name.to_string();
        let flash = FlashOpts {
            base: chip.flash_base,
            size: chip.flash_size,
            sections: vec![FlashSection {
                name: "app".into(),
                addr: chip.flash_base,
                len: chip.flash_size,
            }],
            verify: !args.no_verify,
        };
        return Ok((chip, flash));
    }

    let config = config.ok_or_else(|| {
        anyhow::anyhow!(
            "no --chip argument and no .debugger/chip.toml found. Use --chip <NAME> or run 'mcu-bridge init --chip <NAME>' first."
        )
    })?;

    let mut flash = config.flash.clone();
    flash.verify = !args.no_verify;
    Ok((config.chip.clone(), flash))
}

fn render_human_report(report: &FlashReport) {
    let backend = report.backend.as_deref().unwrap_or("unknown");
    println!("chip: {}", report.chip);
    println!("backend: {backend}");
    println!("verify: {}", report.verify);
    println!("run: {}", report.run);
    for stage in &report.stages {
        let status = if stage.success { "ok" } else { "error" };
        println!("stage {}: {status}", stage_name(stage.stage));
    }
    if report.success {
        println!("[OK] flash complete");
        return;
    }

    if report.attempts.len() > 1 {
        println!("attempts:");
        for attempt in &report.attempts {
            let status = if attempt.success { "ok" } else { "error" };
            println!(
                "  {}: {} at {}",
                attempt.backend,
                status,
                stage_name(attempt.last_stage)
            );
        }
    }

    if let Some(error) = &report.error {
        println!("[ERROR] {} ({})", error.message, error.code);
    }
}

fn stage_name(stage: OperationStage) -> &'static str {
    match stage {
        OperationStage::AttachProbe => "attach_probe",
        OperationStage::ConnectTarget => "connect_target",
        OperationStage::EraseFlash => "erase_flash",
        OperationStage::ProgramFlash => "program_flash",
        OperationStage::VerifyFlash => "verify_flash",
        OperationStage::ResetTarget => "reset_target",
        OperationStage::RunTarget => "run_target",
    }
}

#[cfg(test)]
mod tests {
    use super::{FlashArgs, resolve_chip_and_flash};
    use std::path::PathBuf;

    #[test]
    fn flash_elf_not_found() {
        let args = FlashArgs {
            elf: PathBuf::from("nonexistent.elf"),
            no_verify: false,
            chip: Some("STM32F407VG".into()),
            run: false,
            backend: None,
            openocd_cfg: None,
            json: false,
        };
        let err = super::handle(&args).unwrap_err();
        assert!(err.to_string().contains("ELF file not found"));
    }

    #[test]
    fn resolve_chip_honors_no_verify() {
        let args = FlashArgs {
            elf: PathBuf::from("Cargo.toml"),
            no_verify: true,
            chip: Some("STM32F411RE".into()),
            run: false,
            backend: None,
            openocd_cfg: None,
            json: false,
        };
        let (_, flash) = resolve_chip_and_flash(&args, None).unwrap();
        assert!(!flash.verify);
    }
}
