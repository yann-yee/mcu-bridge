//! flash 子命令 — 烧录 ELF 固件到目标芯片。
//!
//! 设计文档 §4.1：`mcu-bridge flash --elf target/firmware.elf [--verify]`
//!
//! P0 阶段: dry-run — 仅校验参数 + 输出诊断信息，实际烧录留 P1。

use std::path::PathBuf;

/// flash 子命令参数
pub struct FlashArgs {
    pub elf: PathBuf,
    pub verify: bool,
    pub chip: Option<String>,
}

/// 处理 flash 子命令 (P0 dry-run)
pub fn handle(args: &FlashArgs) -> anyhow::Result<()> {
    // 校验 ELF 文件存在
    if !args.elf.exists() {
        anyhow::bail!("ELF file not found: {}", args.elf.display());
    }

    let chip = args.chip.as_deref().unwrap_or("(auto-detect)");

    println!(
        "flash: ELF={}, chip={}, verify={}",
        args.elf.display(),
        chip,
        args.verify
    );
    println!("[INFO] P0 dry-run — actual flash will be supported in P1");

    Ok(())
}
