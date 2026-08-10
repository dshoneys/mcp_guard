//! Presentation shell: tray menu model + console / native tray.
//! Depends on contracts + ui config files only (no scan/watch imports).

#[cfg(any(windows, target_os = "macos"))]
mod native;
#[cfg(any(windows, target_os = "macos"))]
pub use native::{run_native_tray, NativeTrayConfig, NativeTrayHooks};

use crate::contracts::{
    AlertSnapshot, GuardSeverity, TrayActionId, TrayMenuItem, TrayMenuModel,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Deserialize)]
pub struct UiFileConfig {
    #[serde(default)]
    pub tray: TrayCopySection,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrayCopySection {
    #[serde(default)]
    pub copy: TrayCopy,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrayCopy {
    #[serde(default = "default_idle")]
    pub idle: String,
    #[serde(default = "default_exposure")]
    pub exposure: String,
    #[serde(default = "default_activity")]
    pub activity: String,
}

fn default_idle() -> String {
    "MCP Guard — OK".into()
}
fn default_exposure() -> String {
    "Exposure alert".into()
}
fn default_activity() -> String {
    "Suspicious activity".into()
}

impl Default for TrayCopy {
    fn default() -> Self {
        Self {
            idle: default_idle(),
            exposure: default_exposure(),
            activity: default_activity(),
        }
    }
}

impl Default for TrayCopySection {
    fn default() -> Self {
        Self {
            copy: TrayCopy::default(),
        }
    }
}

impl Default for UiFileConfig {
    fn default() -> Self {
        Self {
            tray: TrayCopySection::default(),
        }
    }
}

/// Load `ui/default.toml` (or override path). Missing file → built-in defaults.
pub fn load_ui_config(explicit: Option<&Path>) -> Result<UiFileConfig> {
    let path = explicit
        .map(PathBuf::from)
        .or_else(|| {
            let p = PathBuf::from("ui/default.toml");
            p.exists().then_some(p)
        });
    match path {
        None => Ok(UiFileConfig::default()),
        Some(p) => {
            let raw = std::fs::read_to_string(&p)
                .with_context(|| format!("read ui config {}", p.display()))?;
            // File uses [tray.copy]; flatten via nested struct matching toml
            #[derive(Deserialize)]
            struct File {
                #[serde(default)]
                tray: TrayCopySection,
            }
            let f: File = toml::from_str(&raw)
                .with_context(|| format!("parse ui config {}", p.display()))?;
            Ok(UiFileConfig { tray: f.tray })
        }
    }
}

/// UX state id: idle | exposure | activity (mute forces idle chrome).
pub fn derive_state_id(snap: &AlertSnapshot, muted: bool) -> &'static str {
    if muted {
        return "idle";
    }
    if snap.activity_count > 0 {
        "activity"
    } else if snap.exposure_count > 0 {
        "exposure"
    } else {
        "idle"
    }
}

pub fn severity_for_state(state_id: &str) -> GuardSeverity {
    match state_id {
        "activity" => GuardSeverity::Danger,
        "exposure" => GuardSeverity::Warn,
        _ => GuardSeverity::Ok,
    }
}

pub fn build_menu(
    snap: &AlertSnapshot,
    audit_path: &Path,
    copy: &TrayCopy,
    muted: bool,
) -> TrayMenuModel {
    let state_id = derive_state_id(snap, muted).to_string();
    let severity = severity_for_state(&state_id);
    let mut header_label = match state_id.as_str() {
        "activity" => copy.activity.clone(),
        "exposure" => copy.exposure.clone(),
        _ => copy.idle.clone(),
    };
    if muted {
        header_label = format!("{header_label} (muted)");
    }

    let basename = audit_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| audit_path.display().to_string());

    TrayMenuModel {
        state_id,
        severity,
        header_label,
        muted,
        items: vec![
            TrayMenuItem {
                action: TrayActionId::OpenAudit,
                label: "Open audit log".into(),
                subtitle: Some(basename),
            },
            TrayMenuItem {
                action: TrayActionId::ScanNow,
                label: "Scan now".into(),
                subtitle: None,
            },
            TrayMenuItem {
                action: TrayActionId::Mute,
                label: "Mute alerts (1h)".into(),
                subtitle: None,
            },
            TrayMenuItem {
                action: TrayActionId::Quit,
                label: "Quit".into(),
                subtitle: None,
            },
        ],
    }
}

pub fn mute_until_one_hour_from(now: SystemTime) -> SystemTime {
    now + Duration::from_secs(3600)
}

pub fn is_muted(now: SystemTime, mute_until: Option<SystemTime>) -> bool {
    match mute_until {
        Some(until) => now < until,
        None => false,
    }
}

/// Reveal audit file in the OS file manager (best-effort).
pub fn open_audit(path: &Path) -> Result<()> {
    let path = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf());
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
            .context("spawn explorer to select audit file")?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .context("open -R audit file")?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path.parent().unwrap_or(Path::new(".")))
            .spawn()
            .context("xdg-open audit dir")?;
        Ok(())
    }
}

/// Console stand-in for OS tray (until native tray lands under REQ-TRAY-UI manual QA).
pub fn print_status_json(model: &TrayMenuModel, snap: &AlertSnapshot) -> Result<()> {
    let out = serde_json::json!({
        "menu": model,
        "snapshot": snap,
        "epoch_secs": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}
