use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::backend::create_backend_instance;
use crate::config::{ChipConfig, FlashOpts};
use crate::probe::DebugProbe;

const SCB_CFSR: u32 = 0xE000_ED28;
const SCB_HFSR: u32 = 0xE000_ED2C;
const SCB_BFAR: u32 = 0xE000_ED38;
const SCB_MMFAR: u32 = 0xE000_ED34;

/// A coarse execution stage that can be surfaced to humans and agents.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStage {
    AttachProbe,
    ConnectTarget,
    EraseFlash,
    ProgramFlash,
    VerifyFlash,
    ResetTarget,
    RunTarget,
}

impl OperationStage {
    /// Whether a backend switch remains safe after this stage fails.
    pub fn allows_backend_fallback(self) -> bool {
        matches!(self, Self::AttachProbe | Self::ConnectTarget)
    }
}

/// High-level failure class used in JSON reports.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    Config,
    Parameter,
    Probe,
    Backend,
    Flash,
    Dwarf,
    Serial,
    State,
    Internal,
}

/// A structured failure payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationError {
    pub code: String,
    pub category: FailureCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<OperationStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    pub message: String,
    pub recoverable: bool,
}

/// Per-backend execution summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendAttempt {
    pub backend: String,
    pub last_stage: OperationStage,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<FailureCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub recoverable: bool,
}

/// Stage progress emitted in flash reports.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StageReport {
    pub stage: OperationStage,
    pub success: bool,
}

/// Minimal register summary that is stable across backends.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RegisterSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pc: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sp: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xpsr: Option<u32>,
}

/// Cortex-M fault snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct FaultSummary {
    pub hfsr: u32,
    pub cfsr: u32,
    pub mmfar: u32,
    pub bfar: u32,
}

/// Current target state for status/doctor responses.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TargetSnapshot {
    pub halted: bool,
    pub registers: RegisterSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fault_summary: Option<FaultSummary>,
}

/// Structured flash result for `flash --json`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlashReport {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    pub chip: String,
    pub verify: bool,
    pub run: bool,
    pub stage: OperationStage,
    pub stages: Vec<StageReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
    pub attempts: Vec<BackendAttempt>,
}

/// Per-backend doctor result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendCheck {
    pub backend: String,
    pub attached: bool,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<OperationError>,
}

/// Structured doctor result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorReport {
    pub config_ok: bool,
    pub chip: String,
    pub backend_checks: Vec<BackendCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_backend: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_state: Option<TargetSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registers: Option<RegisterSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fault_summary: Option<FaultSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_error: Option<String>,
}

/// Inputs for a flash workflow run.
#[derive(Debug, Clone)]
pub struct FlashRunOptions {
    pub elf: PathBuf,
    pub chip: ChipConfig,
    pub flash: FlashOpts,
    pub run: bool,
    pub backend_order: Vec<String>,
    pub openocd_cfg: Option<String>,
}

/// Inputs for a doctor workflow run.
#[derive(Debug, Clone)]
pub struct DoctorRunOptions {
    pub chip: ChipConfig,
    pub backend_order: Vec<String>,
    pub openocd_cfg: Option<String>,
    pub config_ok: bool,
    pub config_error: Option<String>,
}

/// Classify an operation failure into a stable code/category pair.
pub fn classify_error(
    message: &str,
    stage: Option<OperationStage>,
    backend: Option<&str>,
) -> OperationError {
    let lowered = message.to_ascii_lowercase();
    let (code, category) = if lowered.contains("dwarf") {
        ("E_NO_DWARF", FailureCategory::Dwarf)
    } else if lowered.contains("serial") || lowered.contains("uart") || lowered.contains("com") {
        ("E_SERIAL", FailureCategory::Serial)
    } else if lowered.contains("probe disconnected") {
        ("E_PROBE", FailureCategory::Probe)
    } else if lowered.contains("recovery failed") {
        ("E_PROBE_LOST", FailureCategory::Probe)
    } else if lowered.contains("invalid")
        || lowered.contains("missing")
        || lowered.contains("unknown backend")
        || lowered.contains("unknown chip")
    {
        ("E_PARAM", FailureCategory::Parameter)
    } else if lowered.contains("cfg file")
        || lowered.contains("config parse")
        || lowered.contains("cannot read")
    {
        ("E_PARAM", FailureCategory::Config)
    } else if matches!(
        stage,
        Some(
            OperationStage::EraseFlash | OperationStage::ProgramFlash | OperationStage::VerifyFlash
        )
    ) || lowered.contains("flash")
        || lowered.contains("verify")
    {
        ("E_FLASH", FailureCategory::Flash)
    } else if lowered.contains("state") || lowered.contains("halt first") {
        ("E_STATE", FailureCategory::State)
    } else {
        ("E_BACKEND", FailureCategory::Backend)
    };

    OperationError {
        code: code.to_string(),
        category,
        stage,
        backend: backend.map(str::to_string),
        message: message.to_string(),
        recoverable: stage.is_some_and(OperationStage::allows_backend_fallback),
    }
}

/// Capture a register summary from the active core.
pub fn capture_register_summary(backend: &mut dyn DebugProbe) -> anyhow::Result<RegisterSummary> {
    let regs = backend.read_regs(None)?;
    Ok(RegisterSummary {
        pc: find_register(&regs, &["pc"]),
        sp: find_register(&regs, &["sp", "msp"]),
        xpsr: find_register(&regs, &["xpsr"]),
    })
}

/// Capture a lightweight target snapshot for status-like commands.
pub fn capture_target_snapshot(backend: &mut dyn DebugProbe) -> TargetSnapshot {
    let halted = backend.poll_halted(None) || backend.is_halted(None);
    let registers = capture_register_summary(backend).unwrap_or_default();
    let fault_summary = capture_fault_summary(backend).ok();
    TargetSnapshot {
        halted,
        registers,
        fault_summary,
    }
}

/// Execute a flash workflow with optional backend fallback before side effects.
pub fn run_flash(options: &FlashRunOptions) -> FlashReport {
    let mut report = FlashReport {
        success: false,
        backend: None,
        chip: options.chip.name.clone(),
        verify: options.flash.verify,
        run: options.run,
        stage: OperationStage::AttachProbe,
        stages: Vec::new(),
        error: None,
        attempts: Vec::new(),
    };

    for backend_name in &options.backend_order {
        let mut stages = Vec::new();

        stages.push(StageReport {
            stage: OperationStage::AttachProbe,
            success: true,
        });
        let mut backend = match create_backend_instance(backend_name, options.openocd_cfg.clone()) {
            Ok(backend) => backend,
            Err(err) => {
                let error = classify_error(
                    &err.to_string(),
                    Some(OperationStage::AttachProbe),
                    Some(backend_name),
                );
                push_failed_attempt(
                    &mut report,
                    backend_name,
                    OperationStage::AttachProbe,
                    error,
                    stages,
                );
                continue;
            }
        };

        let attach_stage = OperationStage::ConnectTarget;
        match backend.attach(&options.chip) {
            Ok(()) => stages.push(StageReport {
                stage: attach_stage,
                success: true,
            }),
            Err(err) => {
                let error =
                    classify_error(&err.to_string(), Some(attach_stage), Some(backend_name));
                push_failed_attempt(&mut report, backend_name, attach_stage, error, stages);
                continue;
            }
        }

        let flash_error_stage = match backend.flash(&options.elf, &options.flash) {
            Ok(()) => None,
            Err(err) => {
                let msg = err.to_string();
                let stage = if msg.to_ascii_lowercase().contains("verify") && options.flash.verify {
                    OperationStage::VerifyFlash
                } else {
                    OperationStage::ProgramFlash
                };
                Some((stage, msg))
            }
        };

        if let Some((stage, msg)) = flash_error_stage {
            let error = classify_error(&msg, Some(stage), Some(backend_name));
            push_failed_attempt(&mut report, backend_name, stage, error, stages);
            let _ = backend.detach();
            return report;
        }

        stages.push(StageReport {
            stage: OperationStage::EraseFlash,
            success: true,
        });
        stages.push(StageReport {
            stage: OperationStage::ProgramFlash,
            success: true,
        });
        if options.flash.verify {
            stages.push(StageReport {
                stage: OperationStage::VerifyFlash,
                success: true,
            });
        }

        if options.run {
            if let Err(err) = backend.reset(None) {
                let error = classify_error(
                    &err.to_string(),
                    Some(OperationStage::ResetTarget),
                    Some(backend_name),
                );
                push_failed_attempt(
                    &mut report,
                    backend_name,
                    OperationStage::ResetTarget,
                    error,
                    stages,
                );
                let _ = backend.detach();
                return report;
            }
            stages.push(StageReport {
                stage: OperationStage::ResetTarget,
                success: true,
            });
            stages.push(StageReport {
                stage: OperationStage::RunTarget,
                success: true,
            });
        }

        if let Err(err) = backend.detach() {
            let stage = if options.run {
                OperationStage::RunTarget
            } else if options.flash.verify {
                OperationStage::VerifyFlash
            } else {
                OperationStage::ProgramFlash
            };
            let error = classify_error(&err.to_string(), Some(stage), Some(backend_name));
            push_failed_attempt(&mut report, backend_name, stage, error, stages);
            return report;
        }

        report.success = true;
        report.backend = Some(backend_name.clone());
        report.stage = if options.run {
            OperationStage::RunTarget
        } else if options.flash.verify {
            OperationStage::VerifyFlash
        } else {
            OperationStage::ProgramFlash
        };
        report.stages = stages;
        report.error = None;
        report.attempts.push(BackendAttempt {
            backend: backend_name.clone(),
            last_stage: report.stage,
            success: true,
            error_code: None,
            category: None,
            message: None,
            recoverable: false,
        });
        return report;
    }

    report
}

/// Execute doctor against one or more candidate backends.
pub fn run_doctor(options: &DoctorRunOptions) -> DoctorReport {
    let mut report = DoctorReport {
        config_ok: options.config_ok,
        chip: options.chip.name.clone(),
        backend_checks: Vec::new(),
        recommended_backend: None,
        target_state: None,
        registers: None,
        fault_summary: None,
        config_error: options.config_error.clone(),
    };

    for backend_name in &options.backend_order {
        let mut check = BackendCheck {
            backend: backend_name.clone(),
            attached: false,
            success: false,
            error: None,
        };

        let mut backend = match create_backend_instance(backend_name, options.openocd_cfg.clone()) {
            Ok(backend) => backend,
            Err(err) => {
                check.error = Some(classify_error(
                    &err.to_string(),
                    Some(OperationStage::AttachProbe),
                    Some(backend_name),
                ));
                report.backend_checks.push(check);
                continue;
            }
        };

        match backend.attach(&options.chip) {
            Ok(()) => {
                check.attached = true;
                check.success = true;
                if report.recommended_backend.is_none() {
                    let snapshot = capture_target_snapshot(&mut *backend);
                    report.registers = Some(snapshot.registers.clone());
                    report.fault_summary = snapshot.fault_summary.clone();
                    report.target_state = Some(snapshot);
                    report.recommended_backend = Some(backend_name.clone());
                }
            }
            Err(err) => {
                check.error = Some(classify_error(
                    &err.to_string(),
                    Some(OperationStage::ConnectTarget),
                    Some(backend_name),
                ));
            }
        }

        let _ = backend.detach();
        report.backend_checks.push(check);
    }

    report
}

fn capture_fault_summary(backend: &mut dyn DebugProbe) -> anyhow::Result<FaultSummary> {
    Ok(FaultSummary {
        hfsr: read_u32(backend, SCB_HFSR)?,
        cfsr: read_u32(backend, SCB_CFSR)?,
        mmfar: read_u32(backend, SCB_MMFAR)?,
        bfar: read_u32(backend, SCB_BFAR)?,
    })
}

fn read_u32(backend: &mut dyn DebugProbe, addr: u32) -> anyhow::Result<u32> {
    let data = backend.read_mem(addr, 4, None)?;
    if data.len() < 4 {
        anyhow::bail!("short read at 0x{addr:08x}");
    }
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

fn find_register(regs: &std::collections::HashMap<String, u64>, names: &[&str]) -> Option<u32> {
    for wanted in names {
        for (name, value) in regs {
            if name.eq_ignore_ascii_case(wanted) {
                return Some(*value as u32);
            }
        }
    }
    None
}

fn push_failed_attempt(
    report: &mut FlashReport,
    backend_name: &str,
    stage: OperationStage,
    error: OperationError,
    mut stages: Vec<StageReport>,
) {
    stages.push(StageReport {
        stage,
        success: false,
    });
    report.backend = Some(backend_name.to_string());
    report.stage = stage;
    report.stages = stages;
    report.error = Some(error.clone());
    report.attempts.push(BackendAttempt {
        backend: backend_name.to_string(),
        last_stage: stage,
        success: false,
        error_code: Some(error.code.clone()),
        category: Some(error.category),
        message: Some(error.message.clone()),
        recoverable: error.recoverable,
    });
}

#[cfg(test)]
mod tests {
    use super::{FailureCategory, OperationStage, classify_error};

    #[test]
    fn classify_flash_errors() {
        let err = classify_error(
            "flash failed: verify mismatch",
            Some(OperationStage::VerifyFlash),
            Some("probe-rs"),
        );
        assert_eq!(err.code, "E_FLASH");
        assert_eq!(err.category, FailureCategory::Flash);
    }

    #[test]
    fn classify_param_errors() {
        let err = classify_error(
            "unknown backend 'x'",
            Some(OperationStage::AttachProbe),
            None,
        );
        assert_eq!(err.code, "E_PARAM");
        assert_eq!(err.category, FailureCategory::Parameter);
    }
}
