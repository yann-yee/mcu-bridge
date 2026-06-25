use std::path::{Path, PathBuf};

use crate::config::AppConfig;
use crate::probe::DebugProbe;
use crate::probe::openocd::OpenOcdBackend;
use crate::probe::probe_rs::ProbeRsBackend;

/// Canonical backend ordering used when `backend = "auto"`.
pub const DEFAULT_BACKEND_ORDER: [&str; 2] = ["probe-rs", "openocd"];

/// Load an app config from an explicit path.
pub fn load_app_config(path: &Path) -> anyhow::Result<AppConfig> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&content).map_err(|e| anyhow::anyhow!("config parse error: {e}"))
}

/// Load `.debugger/chip.toml`.
pub fn load_default_app_config() -> anyhow::Result<AppConfig> {
    load_app_config(Path::new(".debugger/chip.toml"))
}

/// Best-effort load of `.debugger/chip.toml`.
pub fn load_optional_default_app_config() -> Option<AppConfig> {
    load_default_app_config().ok()
}

/// Resolve the backend mode after applying CLI and config overrides.
pub fn resolve_backend_mode(cli_backend: Option<&str>, config: Option<&AppConfig>) -> String {
    cli_backend
        .map(str::to_string)
        .or_else(|| config.map(|cfg| cfg.debugger.backend.clone()))
        .unwrap_or_else(|| "probe-rs".to_string())
}

/// Resolve the OpenOCD cfg path after applying CLI and config overrides.
pub fn resolve_openocd_cfg(
    cli_openocd_cfg: Option<&str>,
    config: Option<&AppConfig>,
) -> Option<String> {
    cli_openocd_cfg
        .map(str::to_string)
        .or_else(|| {
            config
                .and_then(|cfg| cfg.openocd.as_ref())
                .map(|cfg| cfg.cfg_file.clone())
        })
        .or_else(|| {
            let default_path = PathBuf::from(".debugger/openocd.cfg");
            default_path
                .exists()
                .then(|| default_path.to_string_lossy().to_string())
        })
}

/// Resolve the backend order used by flash/doctor auto mode.
pub fn resolve_backend_order(
    mode: &str,
    config: Option<&AppConfig>,
) -> anyhow::Result<Vec<String>> {
    let requested = if mode.eq_ignore_ascii_case("auto") {
        config
            .map(|cfg| cfg.debugger.backend_order.clone())
            .filter(|order| !order.is_empty())
            .unwrap_or_else(crate::config::default_backend_order)
    } else {
        vec![mode.to_string()]
    };

    let mut normalized = Vec::new();
    for backend in requested {
        let backend = normalize_backend_name(&backend)?;
        if !normalized.iter().any(|existing| existing == backend) {
            normalized.push(backend.to_string());
        }
    }

    if normalized.is_empty() {
        anyhow::bail!("backend order cannot be empty");
    }

    Ok(normalized)
}

/// Normalize a backend name to the wire format used by reports.
pub fn normalize_backend_name(name: &str) -> anyhow::Result<&'static str> {
    match name.to_ascii_lowercase().as_str() {
        "probe-rs" | "probers" => Ok("probe-rs"),
        "openocd" => Ok("openocd"),
        "auto" => Ok("auto"),
        other => anyhow::bail!("unknown backend '{other}'. Supported: auto, probe-rs, openocd"),
    }
}

/// Create a concrete backend instance.
pub fn create_backend_instance(
    backend_name: &str,
    openocd_cfg: Option<String>,
) -> anyhow::Result<Box<dyn DebugProbe>> {
    match normalize_backend_name(backend_name)? {
        "probe-rs" => Ok(Box::new(ProbeRsBackend::new())),
        "openocd" => {
            let cfg_path = openocd_cfg.ok_or_else(|| {
                anyhow::anyhow!(
                    "OpenOCD backend requires a config file. Use --openocd-cfg <PATH> or create .debugger/openocd.cfg"
                )
            })?;
            Ok(Box::new(OpenOcdBackend::new(Some(cfg_path))))
        }
        _ => anyhow::bail!("backend 'auto' must be resolved before backend creation"),
    }
}
