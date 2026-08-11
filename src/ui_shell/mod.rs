//! Presentation shell: tray menu model + console / native tray.
//! Depends on contracts + ui config files only (no scan/watch imports).

#[cfg(any(windows, target_os = "macos"))]
mod dashboard;
mod brand;
mod i18n;
#[cfg(any(windows, target_os = "macos"))]
mod native;
#[cfg(windows)]
mod win_process;
pub use brand::brand_icon_rgba;
#[cfg(any(windows, target_os = "macos"))]
pub use dashboard::{run_dashboard, DashboardHooks, DashboardShowHandle};
pub use i18n::{fmt_named, load_catalog, Catalog, DEFAULT_LOCALE};
#[cfg(any(windows, target_os = "macos"))]
pub use native::{run_native_tray, NativeTrayConfig, NativeTrayHooks};
#[cfg(windows)]
pub use win_process::{acquire_tray_singleton, detach_console};

use crate::contracts::{
    AlertSnapshot, GuardSeverity, TrayActionId, TrayMenuItem, TrayMenuModel,
};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct UiBundle {
    pub locale: String,
    pub catalog: Catalog,
    /// Path to ui config file if loaded (for diagnostics).
    pub ui_path: Option<PathBuf>,
}

impl Default for UiBundle {
    fn default() -> Self {
        let (locale, catalog) = load_catalog(DEFAULT_LOCALE).unwrap_or_else(|_| {
            (DEFAULT_LOCALE.into(), Catalog::default())
        });
        Self {
            locale,
            catalog,
            ui_path: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct UiFileRaw {
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    tray: TrayCopySection,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct TrayCopySection {
    #[serde(default)]
    copy: TrayCopyOverrides,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct TrayCopyOverrides {
    idle: Option<String>,
    exposure: Option<String>,
    activity: Option<String>,
}

/// Backward-compatible status labels (tests / callers that only need chrome text).
#[derive(Debug, Clone)]
pub struct TrayCopy {
    pub idle: String,
    pub exposure: String,
    pub activity: String,
}

impl From<&Catalog> for TrayCopy {
    fn from(c: &Catalog) -> Self {
        Self {
            idle: c.status.idle.clone(),
            exposure: c.status.exposure.clone(),
            activity: c.status.activity.clone(),
        }
    }
}

impl Default for TrayCopy {
    fn default() -> Self {
        TrayCopy::from(&Catalog::default())
    }
}

/// Load UI + locale catalog. `locale_override` wins over `ui/default.toml`.
pub fn load_ui_bundle(
    ui_path: Option<&Path>,
    locale_override: Option<&str>,
) -> Result<UiBundle> {
    let path = ui_path
        .map(PathBuf::from)
        .or_else(|| {
            let p = PathBuf::from("ui/default.toml");
            p.exists().then_some(p)
        })
        .or_else(|| {
            let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui/default.toml");
            p.exists().then_some(p)
        });

    let (file_locale, overrides, ui_path) = match path {
        None => (None, TrayCopyOverrides::default(), None),
        Some(p) => {
            let raw = std::fs::read_to_string(&p)
                .with_context(|| format!("read ui config {}", p.display()))?;
            let f: UiFileRaw = toml::from_str(&raw)
                .with_context(|| format!("parse ui config {}", p.display()))?;
            (f.locale, f.tray.copy, Some(p))
        }
    };

    let locale = locale_override
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or(file_locale.as_deref())
        .unwrap_or(DEFAULT_LOCALE);

    let (locale, mut catalog) = load_catalog(locale)?;
    i18n::apply_tray_copy_overrides(
        &mut catalog,
        overrides.idle.as_deref(),
        overrides.exposure.as_deref(),
        overrides.activity.as_deref(),
    );

    Ok(UiBundle {
        locale,
        catalog,
        ui_path,
    })
}

/// Convenience: load with no locale override (file / zh-CN default).
pub fn load_ui_config(explicit: Option<&Path>) -> Result<UiBundle> {
    load_ui_bundle(explicit, None)
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
    catalog: &Catalog,
    muted: bool,
) -> TrayMenuModel {
    let state_id = derive_state_id(snap, muted).to_string();
    let severity = severity_for_state(&state_id);
    let mut header_label = match state_id.as_str() {
        "activity" => catalog.status.activity.clone(),
        "exposure" => catalog.status.exposure.clone(),
        _ => catalog.status.idle.clone(),
    };
    if muted {
        header_label = format!("{header_label}{}", catalog.status.muted_suffix);
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
                action: TrayActionId::OpenDashboard,
                label: catalog.tray.open_dashboard.clone(),
                subtitle: None,
            },
            TrayMenuItem {
                action: TrayActionId::OpenAudit,
                label: catalog.tray.open_audit.clone(),
                subtitle: Some(basename),
            },
            TrayMenuItem {
                action: TrayActionId::ScanNow,
                label: catalog.tray.scan_now.clone(),
                subtitle: None,
            },
            TrayMenuItem {
                action: TrayActionId::Mute,
                label: catalog.tray.mute.clone(),
                subtitle: None,
            },
            TrayMenuItem {
                action: TrayActionId::Quit,
                label: catalog.tray.quit.clone(),
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

/// Best-effort desktop toast (Windows / macOS / Linux). Failures are logged, not fatal.
pub fn notify(summary: &str, body: &str) {
    match notify_rust::Notification::new()
        .appname("MCP Guard")
        .summary(summary)
        .body(body)
        .show()
    {
        Ok(_) => {}
        Err(err) => tracing::warn!(error = %err, "desktop notification failed"),
    }
}

pub fn notify_scan_finished(catalog: &Catalog, open: usize, exposures: usize, activity: usize) {
    let t = &catalog.toast;
    if activity > 0 {
        notify(
            &t.scan_activity_title,
            &fmt_named(&t.scan_activity_body, &[("n", &activity.to_string())]),
        );
    } else if exposures > 0 {
        notify(
            &t.scan_exposure_title,
            &fmt_named(
                &t.scan_exposure_body,
                &[
                    ("exposures", &exposures.to_string()),
                    ("open", &open.to_string()),
                ],
            ),
        );
    } else {
        notify(
            &t.scan_ok_title,
            &fmt_named(&t.scan_ok_body, &[("open", &open.to_string())]),
        );
    }
}

pub fn notify_severity_escalation(catalog: &Catalog, state_id: &str) {
    let t = &catalog.toast;
    match state_id {
        "activity" => notify(&t.scan_activity_title, &t.escalation_activity_body),
        "exposure" => notify(&t.scan_exposure_title, &t.escalation_exposure_body),
        _ => {}
    }
}

pub type SharedCatalog = Arc<Catalog>;
