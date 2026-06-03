//! init 子命令 — 生成 `.debugger/chip.toml` 配置文件。
//!
//! 设计文档 §4.1：`mcu-bridge init --chip STM32F407VG --debugger stlink-v2`
//! 从内置芯片模板库自动填充 Flash/RAM 地址等信息。

use std::fs;

use crate::config::{
    AppConfig, ChipConfig, DebuggerConfig, FlashBpConfig, FlashOpts, FlashSection, RecoveryConfig,
    SerialConfig, WatchConfig,
};

/// init 子命令参数
pub struct InitArgs {
    pub chip: String,
    pub debugger: Option<String>,
    pub interface: Option<String>,
}

/// 内置芯片模板库（P0 硬编码一个模板，P2 扩展为文件模板）
fn get_chip_template(name: &str) -> anyhow::Result<ChipConfig> {
    match name.to_ascii_uppercase().as_str() {
        "STM32F407VG" | "STM32F407" => Ok(ChipConfig {
            name: "STM32F407VG".into(),
            architecture: "cortex-m4".into(),
            flash_base: 0x0800_0000,
            flash_size: 0x0010_0000, // 1MB
            ram_base: 0x2000_0000,
            ram_size: 0x0002_0000, // 128KB
        }),
        unknown => anyhow::bail!("unknown chip '{}'. Available: STM32F407VG", unknown),
    }
}

/// 处理 init 子命令
pub fn handle(args: &InitArgs) -> anyhow::Result<()> {
    let chip = get_chip_template(&args.chip)?;
    let debugger_probe = args.debugger.clone().unwrap_or_else(|| "cmsis-dap".into());
    let debugger_interface = args.interface.clone().unwrap_or_else(|| "swd".into());

    let config = AppConfig {
        chip: chip.clone(),
        debugger: DebuggerConfig {
            probe: debugger_probe.clone(),
            interface: debugger_interface,
            speed_khz: 4000,
            backend: "probe-rs".into(),
        },
        flash: FlashOpts {
            base: chip.flash_base,
            size: chip.flash_size,
            sections: vec![FlashSection {
                name: "app".into(),
                addr: chip.flash_base,
                len: chip.flash_size,
            }],
            verify: true,
        },
        serial: SerialConfig::default(),
        watch: WatchConfig::default(),
        recovery: RecoveryConfig::default(),
        flash_bp: FlashBpConfig::default(),
        openocd: None,
    };

    let toml_str = toml::to_string_pretty(&config)?;
    fs::create_dir_all(".debugger")?;
    fs::write(".debugger/chip.toml", &toml_str)?;
    println!("[INFO] config written to .debugger/chip.toml");
    Ok(())
}
