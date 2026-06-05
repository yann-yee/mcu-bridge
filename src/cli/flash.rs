//! flash 子命令 — 烧录 ELF 固件到目标芯片.
//!
//! 设计文档 §4.1：`mcu-bridge flash --elf target/firmware.elf [--verify] [--run]`
//!
//! Standalone 模式：临时创建 ProbeRsBackend → attach → flash → detach。
//! 芯片配置来源（按优先级）：
//!   1. `--chip` 命令行参数
//!   2. `.debugger/chip.toml` 配置文件

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use crate::cli::init;
use crate::config::{AppConfig, ChipConfig, FlashOpts, FlashSection};
use crate::probe::DebugProbe;
use crate::probe::openocd::OpenOcdBackend;
use crate::probe::probe_rs::ProbeRsBackend;

/// flash 子命令参数
pub struct FlashArgs {
    pub elf: PathBuf,
    pub verify: bool,
    pub chip: Option<String>,
    pub run: bool,
    pub backend: Option<String>,
    pub openocd_cfg: Option<String>,
}

/// 从 `.debugger/chip.toml` 加载完整配置。
fn load_config_from_dot_debugger() -> anyhow::Result<AppConfig> {
    let path = Path::new(".debugger/chip.toml");
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
    let config: AppConfig =
        toml::from_str(&content).map_err(|e| anyhow::anyhow!("config parse error: {e}"))?;
    Ok(config)
}

/// 解析芯片配置（CLI 参数优先，否则读取配置文件）。
fn resolve_chip_config(chip_arg: Option<&str>) -> anyhow::Result<(ChipConfig, FlashOpts)> {
    if let Some(name) = chip_arg {
        // 先用模板校验芯片存在并获取架构信息
        let mut chip = init::get_chip_template(name)?;
        // 用用户输入的精确芯片名（probe-rs 需要精确的 target 名称）
        chip.name = name.to_string();
        let opts = FlashOpts {
            base: chip.flash_base,
            size: chip.flash_size,
            sections: vec![FlashSection {
                name: "app".into(),
                addr: chip.flash_base,
                len: chip.flash_size,
            }],
            verify: true,
        };
        return Ok((chip, opts));
    }

    // 回退：尝试读取 .debugger/chip.toml
    let app = load_config_from_dot_debugger()?;
    Ok((app.chip, app.flash))
}

/// 创建探测后端（根据 CLI 参数或 TOML 配置选择）。
///
/// 优先级：`--backend` CLI 参数 > `.debugger/chip.toml` 中 `[debugger].backend` 字段 > 缺省 probe-rs
/// 配置路径：`--openocd-cfg` CLI 参数 > `.debugger/chip.toml` 中 `[openocd].cfg_file` 字段 > `.debugger/openocd.cfg` 兜底
fn create_backend(args: &FlashArgs) -> anyhow::Result<Box<dyn DebugProbe>> {
    // 先从 TOML 加载配置（若存在），用于读取后端类型和 openocd 配置
    let config_from_toml = load_config_from_dot_debugger().ok();

    // 确定后端类型
    let backend_type = match &args.backend {
        Some(val) => val.clone(),
        None => match &config_from_toml {
            Some(cfg) => cfg.debugger.backend.clone(),
            None => "probe-rs".to_string(),
        },
    };

    // 确定 OpenOCD 配置文件路径
    let openocd_cfg = args.openocd_cfg.clone().or_else(|| {
        config_from_toml
            .as_ref()
            .and_then(|cfg| cfg.openocd.as_ref())
            .map(|o| o.cfg_file.clone())
    });

    match backend_type.to_ascii_lowercase().as_str() {
        "probe-rs" => Ok(Box::new(ProbeRsBackend::new())),
        "openocd" => Ok(Box::new(OpenOcdBackend::new(openocd_cfg))),
        _ => anyhow::bail!("unknown backend '{backend_type}'. Supported: probe-rs, openocd"),
    }
}

/// 处理 flash 子命令 (Standalone 烧录流程)
pub fn handle(args: &FlashArgs) -> anyhow::Result<()> {
    // 校验 ELF 文件存在
    if !args.elf.exists() {
        anyhow::bail!("ELF file not found: {}", args.elf.display());
    }

    // 解析芯片配置
    let (chip, flash_opts) = resolve_chip_config(args.chip.as_deref())?;

    eprintln!("[INFO] attaching probe to {}...", chip.name);

    // Standalone 烧录流程（后端不可知）
    let mut backend = create_backend(args)?;
    backend.attach(&chip)?;

    eprintln!("[INFO] flashing ELF...");
    backend.flash(&args.elf, &flash_opts)?;

    if args.run {
        eprintln!("[INFO] resuming target...");
        backend.resume(None)?;
    }

    backend.detach()?;
    println!("[OK] flash complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{FlashArgs, create_backend, handle};
    use std::path::PathBuf;

    #[test]
    fn test_flash_elf_not_found() {
        let args = FlashArgs {
            elf: PathBuf::from("nonexistent.elf"),
            verify: true,
            chip: Some("STM32F407VG".into()),
            run: false,
            backend: None,
            openocd_cfg: None,
        };
        let err = handle(&args).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("ELF file not found"), "got: {msg}");
    }

    #[test]
    fn test_flash_unknown_chip() {
        let args = FlashArgs {
            elf: PathBuf::from("Cargo.toml"),
            verify: true,
            chip: Some("INVALID".into()),
            run: false,
            backend: None,
            openocd_cfg: None,
        };
        let err = handle(&args).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown chip"), "got: {msg}");
    }

    #[test]
    fn test_flash_no_chip_no_config() {
        // 确保 .debugger/chip.toml 不存在
        let args = FlashArgs {
            elf: PathBuf::from("Cargo.toml"),
            verify: true,
            chip: None,
            run: false,
            backend: None,
            openocd_cfg: None,
        };
        let err = handle(&args).unwrap_err();
        let msg = err.to_string();
        // 没有 --chip 也没有 .debugger/chip.toml 时应提示需要 chip
        assert!(
            msg.contains("cannot read") || msg.contains("not found") || msg.contains("chip"),
            "got: {msg}"
        );
    }

    #[test]
    fn test_flash_backend_probe_rs_default() {
        // 默认无 --backend → create_backend 成功返回（不关心具体类型）
        let args = FlashArgs {
            elf: PathBuf::from("Cargo.toml"),
            verify: true,
            chip: Some("STM32F407VG".into()),
            run: false,
            backend: None,
            openocd_cfg: None,
        };
        assert!(
            create_backend(&args).is_ok(),
            "default backend should be Ok"
        );
    }

    #[test]
    fn test_flash_backend_openocd_no_cfg() {
        // --backend openocd 但无配置文件 → attach 阶段报 cfg not found
        let args = FlashArgs {
            elf: PathBuf::from("Cargo.toml"),
            verify: true,
            chip: Some("STM32F407VG".into()),
            run: false,
            backend: Some("openocd".into()),
            openocd_cfg: None,
        };
        let err = handle(&args).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("cfg file not found"),
            "expected cfg file not found, got: {msg}"
        );
    }

    #[test]
    fn test_flash_backend_unknown() {
        // --backend invalid → 在 create_backend 阶段报错
        let args = FlashArgs {
            elf: PathBuf::from("Cargo.toml"),
            verify: true,
            chip: Some("STM32F407VG".into()),
            run: false,
            backend: Some("invalid".into()),
            openocd_cfg: None,
        };
        let err = handle(&args).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown backend"),
            "expected unknown backend, got: {msg}"
        );
    }
}
