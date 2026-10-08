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
    } else if lowered.contains("serial")
        || lowered.contains("uart")
        || contains_windows_com_port(&lowered)
    {
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

fn contains_windows_com_port(message: &str) -> bool {
    message
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|word| {
            word.strip_prefix("com").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
            })
        })
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
    run_flash_with_factory(options, |backend_name, openocd_cfg| {
        create_backend_instance(backend_name, openocd_cfg)
    })
}

fn run_flash_with_factory<F>(options: &FlashRunOptions, mut create_backend: F) -> FlashReport
where
    F: FnMut(&str, Option<String>) -> anyhow::Result<Box<dyn DebugProbe>>,
{
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

        let mut backend = match create_backend(backend_name, options.openocd_cfg.clone()) {
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

        stages.push(StageReport {
            stage: OperationStage::AttachProbe,
            success: true,
        });

        let attach_stage = OperationStage::ConnectTarget;
        match backend.attach(&options.chip) {
            Ok(()) => stages.push(StageReport {
                stage: attach_stage,
                success: true,
            }),
            Err(err) => {
                let error =
                    classify_error(&err.to_string(), Some(attach_stage), Some(backend_name));
                let _ = backend.detach();
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
            stage: if options.flash.verify {
                OperationStage::VerifyFlash
            } else {
                OperationStage::ProgramFlash
            },
            success: true,
        });

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
    run_doctor_with_factory(options, |backend_name, openocd_cfg| {
        create_backend_instance(backend_name, openocd_cfg)
    })
}

fn run_doctor_with_factory<F>(options: &DoctorRunOptions, mut create_backend: F) -> DoctorReport
where
    F: FnMut(&str, Option<String>) -> anyhow::Result<Box<dyn DebugProbe>>,
{
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

        let mut backend = match create_backend(backend_name, options.openocd_cfg.clone()) {
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

        if let Err(err) = backend.detach()
            && check.error.is_none()
        {
            check.success = false;
            check.error = Some(classify_error(&err.to_string(), None, Some(backend_name)));
        }
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
    use std::collections::HashMap;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use crate::config::{ChipConfig, FlashOpts};
    use crate::probe::{BpId, DebugProbe, WatchKind, WpId};

    use super::{
        DoctorRunOptions, FailureCategory, FlashRunOptions, OperationStage, classify_error,
        run_doctor_with_factory, run_flash_with_factory,
    };

    struct MockProbe {
        name: &'static str,
        events: Arc<Mutex<Vec<String>>>,
        attach_error: Option<&'static str>,
        flash_error: Option<&'static str>,
        reset_error: Option<&'static str>,
        detach_error: Option<&'static str>,
    }

    impl MockProbe {
        fn record(&self, event: impl AsRef<str>) {
            self.events
                .lock()
                .expect("mock event mutex poisoned")
                .push(event.as_ref().to_string());
        }

        fn fail(message: Option<&'static str>) -> anyhow::Result<()> {
            if let Some(message) = message {
                anyhow::bail!(message);
            }
            Ok(())
        }
    }

    impl DebugProbe for MockProbe {
        fn backend_name(&self) -> &'static str {
            self.name
        }

        fn attach(&mut self, _chip: &ChipConfig) -> anyhow::Result<()> {
            self.record(format!("attach:{}", self.name));
            Self::fail(self.attach_error)
        }

        fn detach(&mut self) -> anyhow::Result<()> {
            self.record(format!("detach:{}", self.name));
            Self::fail(self.detach_error)
        }

        fn is_connected(&self) -> bool {
            true
        }

        fn try_recover(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        fn flash(&mut self, _elf: &Path, opts: &FlashOpts) -> anyhow::Result<()> {
            self.record(format!("flash:{}:verify={}", self.name, opts.verify));
            Self::fail(self.flash_error)
        }

        fn halt(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
            Ok(())
        }

        fn resume(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
            Ok(())
        }

        fn step(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
            Ok(())
        }

        fn reset(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
            self.record(format!("reset:{}", self.name));
            Self::fail(self.reset_error)
        }

        fn core_count(&self) -> usize {
            1
        }

        fn active_core(&self) -> usize {
            0
        }

        fn set_breakpoint(&mut self, _addr: u32, _core: Option<usize>) -> anyhow::Result<BpId> {
            Ok(0)
        }

        fn clear_breakpoint(&mut self, _id: BpId) -> anyhow::Result<()> {
            Ok(())
        }

        fn set_watchpoint(
            &mut self,
            _addr: u32,
            _len: u32,
            _kind: WatchKind,
        ) -> anyhow::Result<WpId> {
            Ok(0)
        }

        fn clear_watchpoint(&mut self, _id: WpId) -> anyhow::Result<()> {
            Ok(())
        }

        fn read_mem(
            &mut self,
            addr: u32,
            len: u32,
            _core: Option<usize>,
        ) -> anyhow::Result<Vec<u8>> {
            let value: u32 = match addr {
                0xE000_ED28 => 0x0000_8200,
                0xE000_ED2C => 0x8000_0000,
                0xE000_ED34 => 0x2000_1234,
                0xE000_ED38 => 0x0800_5678,
                _ => 0,
            };
            Ok(value.to_le_bytes()[..len as usize].to_vec())
        }

        fn write_mem(
            &mut self,
            _addr: u32,
            _data: &[u8],
            _core: Option<usize>,
        ) -> anyhow::Result<()> {
            Ok(())
        }

        fn read_regs(&mut self, _core: Option<usize>) -> anyhow::Result<HashMap<String, u64>> {
            Ok(HashMap::from([
                ("PC".to_string(), 0x0800_1234),
                ("msp".to_string(), 0x2000_5678),
                ("xPSR".to_string(), 0x6100_0000),
            ]))
        }

        fn is_halted(&mut self, _core: Option<usize>) -> bool {
            true
        }
    }

    fn mock_probe(
        name: &'static str,
        events: &Arc<Mutex<Vec<String>>>,
        attach_error: Option<&'static str>,
        flash_error: Option<&'static str>,
        reset_error: Option<&'static str>,
        detach_error: Option<&'static str>,
    ) -> Box<dyn DebugProbe> {
        Box::new(MockProbe {
            name,
            events: Arc::clone(events),
            attach_error,
            flash_error,
            reset_error,
            detach_error,
        })
    }

    fn flash_options(backend_order: &[&str], run: bool) -> FlashRunOptions {
        FlashRunOptions {
            elf: "firmware.elf".into(),
            chip: ChipConfig {
                name: "test-chip".into(),
                architecture: "cortex-m4".into(),
                flash_base: 0x0800_0000,
                flash_size: 0x1_0000,
                ram_base: 0x2000_0000,
                ram_size: 0x1_0000,
            },
            flash: FlashOpts {
                base: 0x0800_0000,
                size: 0x1_0000,
                sections: Vec::new(),
                verify: true,
            },
            run,
            backend_order: backend_order.iter().map(|name| (*name).into()).collect(),
            openocd_cfg: None,
        }
    }

    fn doctor_options(backend_order: &[&str]) -> DoctorRunOptions {
        DoctorRunOptions {
            chip: flash_options(&[], false).chip,
            backend_order: backend_order.iter().map(|name| (*name).into()).collect(),
            openocd_cfg: None,
            config_ok: true,
            config_error: None,
        }
    }

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

    #[test]
    fn communication_and_command_errors_are_not_serial_errors() {
        for message in ["communication link failed", "command execution failed"] {
            let err = classify_error(message, Some(OperationStage::ConnectTarget), None);
            assert_eq!(err.code, "E_BACKEND", "{message}");
            assert_eq!(err.category, FailureCategory::Backend, "{message}");
        }

        let serial = classify_error("could not open COM3", None, None);
        assert_eq!(serial.code, "E_SERIAL");
        assert_eq!(serial.category, FailureCategory::Serial);
    }

    #[test]
    fn flash_uses_default_verification_then_resets_before_detach() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let factory_events = Arc::clone(&events);
        let report = run_flash_with_factory(&flash_options(&["probe-rs"], true), move |name, _| {
            factory_events
                .lock()
                .expect("mock event mutex poisoned")
                .push(format!("create:{name}"));
            Ok(mock_probe(
                "probe-rs",
                &factory_events,
                None,
                None,
                None,
                None,
            ))
        });

        assert!(report.success);
        assert!(report.verify);
        assert_eq!(
            *events.lock().expect("mock event mutex poisoned"),
            [
                "create:probe-rs",
                "attach:probe-rs",
                "flash:probe-rs:verify=true",
                "reset:probe-rs",
                "detach:probe-rs",
            ]
        );
        assert_eq!(report.stage, OperationStage::RunTarget);
    }

    #[test]
    fn attach_failure_detaches_before_safe_fallback() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let factory_events = Arc::clone(&events);
        let report = run_flash_with_factory(
            &flash_options(&["probe-rs", "openocd"], false),
            move |name, _| {
                factory_events
                    .lock()
                    .expect("mock event mutex poisoned")
                    .push(format!("create:{name}"));
                let attach_error = (name == "probe-rs").then_some("connection failed");
                Ok(mock_probe(
                    if name == "probe-rs" {
                        "probe-rs"
                    } else {
                        "openocd"
                    },
                    &factory_events,
                    attach_error,
                    None,
                    None,
                    None,
                ))
            },
        );

        assert!(report.success);
        assert_eq!(report.backend.as_deref(), Some("openocd"));
        assert_eq!(report.attempts.len(), 2);
        assert_eq!(report.attempts[0].last_stage, OperationStage::ConnectTarget);
        assert!(report.attempts[0].recoverable);
        assert!(report.attempts[1].success);
        assert_eq!(
            *events.lock().expect("mock event mutex poisoned"),
            [
                "create:probe-rs",
                "attach:probe-rs",
                "detach:probe-rs",
                "create:openocd",
                "attach:openocd",
                "flash:openocd:verify=true",
                "detach:openocd",
            ]
        );
    }

    #[test]
    fn verification_failure_is_not_successful_or_retried_and_detaches() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let factory_events = Arc::clone(&events);
        let report = run_flash_with_factory(
            &flash_options(&["probe-rs", "openocd"], true),
            move |name, _| {
                factory_events
                    .lock()
                    .expect("mock event mutex poisoned")
                    .push(format!("create:{name}"));
                Ok(mock_probe(
                    if name == "probe-rs" {
                        "probe-rs"
                    } else {
                        "openocd"
                    },
                    &factory_events,
                    None,
                    Some("verify mismatch"),
                    None,
                    None,
                ))
            },
        );

        assert!(!report.success);
        assert_eq!(report.stage, OperationStage::VerifyFlash);
        let error = report.error.expect("verification error missing");
        assert_eq!(error.code, "E_FLASH");
        assert_eq!(error.stage, Some(OperationStage::VerifyFlash));
        assert_eq!(
            *events.lock().expect("mock event mutex poisoned"),
            [
                "create:probe-rs",
                "attach:probe-rs",
                "flash:probe-rs:verify=true",
                "detach:probe-rs",
            ]
        );
    }

    #[test]
    fn program_failure_detaches_without_fallback_or_reset() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let factory_events = Arc::clone(&events);
        let report = run_flash_with_factory(
            &flash_options(&["probe-rs", "openocd"], true),
            move |name, _| {
                factory_events
                    .lock()
                    .expect("mock event mutex poisoned")
                    .push(format!("create:{name}"));
                Ok(mock_probe(
                    if name == "probe-rs" {
                        "probe-rs"
                    } else {
                        "openocd"
                    },
                    &factory_events,
                    None,
                    Some("program failed"),
                    None,
                    None,
                ))
            },
        );

        assert!(!report.success);
        assert_eq!(report.stage, OperationStage::ProgramFlash);
        let error = report.error.as_ref().expect("program error missing");
        assert_eq!(error.code, "E_FLASH");
        assert_eq!(error.stage, Some(OperationStage::ProgramFlash));
        assert_eq!(report.attempts.len(), 1);
        assert!(!report.attempts[0].recoverable);
        assert_eq!(
            *events.lock().expect("mock event mutex poisoned"),
            [
                "create:probe-rs",
                "attach:probe-rs",
                "flash:probe-rs:verify=true",
                "detach:probe-rs",
            ]
        );
    }

    #[test]
    fn reset_failure_is_reported_and_detaches_without_claiming_run_success() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let factory_events = Arc::clone(&events);
        let report =
            run_flash_with_factory(&flash_options(&["probe-rs"], true), move |_name, _| {
                Ok(mock_probe(
                    "probe-rs",
                    &factory_events,
                    None,
                    None,
                    Some("reset failed"),
                    None,
                ))
            });

        assert!(!report.success);
        assert_eq!(report.stage, OperationStage::ResetTarget);
        assert_eq!(
            report.error.as_ref().unwrap().stage,
            Some(OperationStage::ResetTarget)
        );
        assert!(
            !report
                .stages
                .iter()
                .any(|stage| { stage.stage == OperationStage::RunTarget && stage.success })
        );
        assert_eq!(
            *events.lock().expect("mock event mutex poisoned"),
            [
                "attach:probe-rs",
                "flash:probe-rs:verify=true",
                "reset:probe-rs",
                "detach:probe-rs",
            ]
        );
    }

    #[test]
    fn doctor_reports_factory_attach_and_detach_diagnostics() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let factory_events = Arc::clone(&events);
        let report =
            run_doctor_with_factory(&doctor_options(&["probe-rs", "openocd"]), move |name, _| {
                if name == "probe-rs" {
                    anyhow::bail!("communication setup failed");
                }
                Ok(mock_probe(
                    "openocd",
                    &factory_events,
                    Some("probe disconnected"),
                    None,
                    None,
                    Some("cleanup failed"),
                ))
            });

        assert_eq!(report.backend_checks.len(), 2);
        assert_eq!(
            report.backend_checks[0].error.as_ref().unwrap().code,
            "E_BACKEND"
        );
        assert_eq!(
            report.backend_checks[1].error.as_ref().unwrap().code,
            "E_PROBE"
        );
        assert!(!report.backend_checks[1].success);
        assert_eq!(
            *events.lock().expect("mock event mutex poisoned"),
            ["attach:openocd", "detach:openocd"]
        );
    }

    #[test]
    fn doctor_reports_detach_failure_after_successful_attach() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let factory_events = Arc::clone(&events);
        let report = run_doctor_with_factory(&doctor_options(&["probe-rs"]), move |_name, _| {
            Ok(mock_probe(
                "probe-rs",
                &factory_events,
                None,
                None,
                None,
                Some("cleanup failed"),
            ))
        });

        let check = &report.backend_checks[0];
        assert!(check.attached);
        assert!(!check.success);
        let error = check.error.as_ref().expect("detach error missing");
        assert_eq!(error.code, "E_BACKEND");
        assert_eq!(error.stage, None);
        assert_eq!(report.recommended_backend.as_deref(), Some("probe-rs"));
        assert_eq!(
            report.registers,
            Some(super::RegisterSummary {
                pc: Some(0x0800_1234),
                sp: Some(0x2000_5678),
                xpsr: Some(0x6100_0000),
            })
        );
        let snapshot = report
            .target_state
            .as_ref()
            .expect("target snapshot missing");
        assert!(snapshot.halted);
        assert_eq!(
            snapshot.fault_summary,
            Some(super::FaultSummary {
                hfsr: 0x8000_0000,
                cfsr: 0x0000_8200,
                mmfar: 0x2000_1234,
                bfar: 0x0800_5678,
            })
        );
        assert_eq!(
            *events.lock().expect("mock event mutex poisoned"),
            ["attach:probe-rs", "detach:probe-rs"]
        );
    }

    #[test]
    fn backend_factory_failure_is_not_reported_as_probe_attached() {
        let report = run_flash_with_factory(&flash_options(&["probe-rs"], false), |_, _| {
            anyhow::bail!("command setup failed")
        });

        assert!(!report.success);
        assert_eq!(report.stage, OperationStage::AttachProbe);
        assert_eq!(report.stages.len(), 1);
        assert_eq!(report.stages[0].stage, OperationStage::AttachProbe);
        assert!(!report.stages[0].success);
        assert_eq!(report.error.as_ref().unwrap().code, "E_BACKEND");
    }
}
