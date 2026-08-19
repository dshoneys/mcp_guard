//! Main dashboard window (WebView) — Code-as-Design HTML under ui/preview/REQ-MAIN-UI.

use crate::contracts::{AlertSnapshot, TickSummary};
use crate::ui_shell::{
    derive_state_id, fmt_named, notify_scan_finished, open_audit, Catalog,
};
use crate::vault::{SecretMeta, Vault};
use anyhow::{bail, Context, Result};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::run_return::EventLoopExtRunReturn;
use tao::window::{Icon as WindowIcon, WindowBuilder};
#[cfg(windows)]
use tao::platform::windows::{EventLoopBuilderExtWindows, WindowBuilderExtWindows};
use wry::http;
use wry::WebViewBuilder;

pub struct DashboardHooks {
    pub scan: Arc<dyn Fn() -> Result<TickSummary> + Send + Sync>,
    pub status: Arc<dyn Fn() -> Result<(AlertSnapshot, bool)> + Send + Sync>,
    /// Latest risk lines for the scan panel (usually from audit).
    pub risks: Arc<dyn Fn() -> Result<Vec<crate::contracts::RiskDetail>> + Send + Sync>,
    /// Add a process to the manual allowlist (persisted on disk).
    pub allow_process: Arc<dyn Fn(&str) -> Result<String> + Send + Sync>,
    pub audit_path: PathBuf,
    pub catalog: Arc<Catalog>,
    pub mute_until: Arc<Mutex<Option<SystemTime>>>,
    pub vault: Arc<Vault>,
    /// When true (tray mode): minimize/close hide the window instead of exiting.
    pub hide_to_tray: bool,
    /// Filled on dashboard Init so the tray can restore a hidden window.
    pub show_handle: Arc<Mutex<Option<DashboardShowHandle>>>,
}

/// Cross-thread restore for a running dashboard event loop.
#[derive(Clone)]
pub struct DashboardShowHandle {
    proxy: tao::event_loop::EventLoopProxy<UserEvent>,
}

impl DashboardShowHandle {
    pub fn show(&self) {
        let _ = self.proxy.send_event(UserEvent::ShowWindow);
    }

    pub fn request_exit(&self) {
        let _ = self.proxy.send_event(UserEvent::ExitLoop);
    }
}

enum UserEvent {
    Refresh,
    ScanDone(TickSummary),
    ScanFailed(String),
    WindowMinimize,
    WindowToggleMaximize,
    WindowClose,
    ShowWindow,
    ExitLoop,
}

fn ui_root_dir() -> Result<PathBuf> {
    let candidates = [
        PathBuf::from("ui"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("ui"),
    ];
    for p in candidates {
        if p.join("preview/REQ-MAIN-UI/index.html").is_file() {
            return Ok(p);
        }
    }
    bail!("dashboard HTML not found (ui/preview/REQ-MAIN-UI/index.html)")
}

fn mime_for(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "json" => "application/json",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// Serve `ui/` over a custom scheme so IPC Source is a valid http URI.
/// (`file://` pages crash wry 0.53 on Windows: Request::builder().uri(file…).unwrap())
fn serve_ui_asset(
    ui_root: &Path,
    request: &http::Request<Vec<u8>>,
) -> http::Response<std::borrow::Cow<'static, [u8]>> {
    use std::borrow::Cow;

    fn resp(status: u16, body: Cow<'static, [u8]>) -> http::Response<Cow<'static, [u8]>> {
        http::Response::builder()
            .status(status)
            .body(body)
            .unwrap_or_else(|_| http::Response::new(Cow::Borrowed(b"".as_slice())))
    }

    let path = request.uri().path();
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() {
        "preview/REQ-MAIN-UI/index.html"
    } else {
        rel
    };
    let joined = ui_root.join(rel);
    let Ok(canon_root) = ui_root.canonicalize() else {
        return resp(500, Cow::Borrowed(b"ui root missing"));
    };
    let file = joined.canonicalize().ok();
    let allowed = file
        .as_ref()
        .is_some_and(|f| f.starts_with(&canon_root) && f.is_file());
    if !allowed {
        return resp(404, Cow::Owned(format!("not found: {rel}").into_bytes()));
    }
    let file = file.unwrap();
    match std::fs::read(&file) {
        Ok(bytes) => http::Response::builder()
            .status(200)
            .header(http::header::CONTENT_TYPE, mime_for(&file))
            .body(Cow::Owned(bytes))
            .unwrap_or_else(|_| http::Response::new(Cow::Borrowed(b"".as_slice()))),
        Err(_) => resp(500, Cow::Borrowed(b"read failed")),
    }
}

fn dashboard_entry_url() -> &'static str {
    // Windows WebView2 maps custom schemes to http://{scheme}.localhost/...
    #[cfg(windows)]
    {
        "http://mcpguard.localhost/preview/REQ-MAIN-UI/index.html"
    }
    #[cfg(not(windows))]
    {
        "mcpguard://localhost/preview/REQ-MAIN-UI/index.html"
    }
}

fn snapshot_js(snap: &AlertSnapshot, muted: bool, audit_path: &Path, catalog: &Catalog) -> String {
    let state = derive_state_id(snap, muted);
    let label = match state {
        "activity" => catalog.status.activity.as_str(),
        "exposure" => catalog.status.exposure.as_str(),
        _ => catalog.status.idle.as_str(),
    };
    let basename = audit_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| audit_path.display().to_string());
    let payload = json!({
        "state_id": state,
        "label": label,
        "muted": muted,
        "muted_suffix": catalog.status.muted_suffix,
        "exposure_count": snap.exposure_count,
        "activity_count": snap.activity_count,
        "last_scan_at": snap.last_scan_at,
        "audit_basename": basename,
    });
    format!(
        "window.mcpGuardApply && window.mcpGuardApply({});",
        payload
    )
}

fn i18n_js(catalog: &Catalog) -> String {
    format!(
        "window.mcpGuardI18n && window.mcpGuardI18n({});",
        catalog.dashboard_json()
    )
}

fn scan_result_js(catalog: &Catalog, summary: &TickSummary) -> String {
    let (kind, headline) = if summary.activity_alerts > 0 {
        ("danger", catalog.dashboard.scan_panel_danger.as_str())
    } else if summary.exposures > 0 {
        ("warn", catalog.dashboard.scan_panel_warn.as_str())
    } else {
        ("ok", catalog.dashboard.scan_panel_ok.as_str())
    };
    let payload = json!({
        "kind": kind,
        "headline": headline,
        "open_services": summary.open_services,
        "exposure_count": summary.exposures,
        "activity_count": summary.activity_alerts,
        "risks": summary.risks,
    });
    format!(
        "window.mcpGuardScanResult && window.mcpGuardScanResult({});",
        payload
    )
}

fn risks_from_audit_js(catalog: &Catalog, snap: &AlertSnapshot, risks: &[crate::contracts::RiskDetail]) -> Option<String> {
    if risks.is_empty() && snap.exposure_count == 0 && snap.activity_count == 0 {
        return None;
    }
    let (kind, headline) = if snap.activity_count > 0 {
        ("danger", catalog.dashboard.scan_panel_danger.as_str())
    } else if snap.exposure_count > 0 {
        ("warn", catalog.dashboard.scan_panel_warn.as_str())
    } else {
        ("ok", catalog.dashboard.scan_panel_ok.as_str())
    };
    let payload = json!({
        "kind": kind,
        "headline": headline,
        "open_services": 0,
        "exposure_count": snap.exposure_count,
        "activity_count": snap.activity_count,
        "risks": risks,
        "from_audit": true,
    });
    Some(format!(
        "window.mcpGuardScanResult && window.mcpGuardScanResult({});",
        payload
    ))
}

fn scan_failed_js(catalog: &Catalog, err: &str) -> String {
    let payload = json!({
        "kind": "error",
        "headline": catalog.dashboard.scan_panel_error,
        "detail": err,
    });
    format!(
        "window.mcpGuardScanResult && window.mcpGuardScanResult({});",
        payload
    )
}

fn vault_list_js(items: &[SecretMeta]) -> String {
    let payload = json!({ "secrets": items });
    format!(
        "window.mcpGuardVaultApply && window.mcpGuardVaultApply({});",
        payload
    )
}

/// Open the main dashboard in a dedicated OS window (blocks until closed).
pub fn run_dashboard(hooks: DashboardHooks) -> Result<()> {
    let ui_root = ui_root_dir()?;

    let mut builder = EventLoopBuilder::<UserEvent>::with_user_event();
    #[cfg(windows)]
    {
        // Tray owns the process main thread; dashboard opens on a worker thread.
        builder.with_any_thread(true);
    }
    let event_loop = builder.build();
    let proxy = event_loop.create_proxy();
    let mut event_loop = event_loop;

    let window_icon = match super::brand::brand_icon_rgba(48) {
        Ok((rgba, w, h)) => WindowIcon::from_rgba(rgba, w, h).ok(),
        Err(err) => {
            tracing::warn!(error = %err, "window icon load failed");
            None
        }
    };

    let mut window_builder = WindowBuilder::new()
        .with_title("MCP Guard")
        .with_inner_size(tao::dpi::LogicalSize::new(820.0, 960.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(600.0, 700.0))
        .with_decorations(false)
        .with_resizable(true);
    if let Some(icon) = window_icon {
        window_builder = window_builder.with_window_icon(Some(icon));
    }
    #[cfg(windows)]
    {
        window_builder = window_builder.with_undecorated_shadow(true);
    }
    let window = window_builder
        .build(&event_loop)
        .context("create dashboard window")?;

    let hooks = Arc::new(hooks);
    let hooks_ipc = Arc::clone(&hooks);
    let proxy_ipc = proxy.clone();
    let scanning = Arc::new(AtomicBool::new(false));
    let scanning_ipc = Arc::clone(&scanning);
    let ui_root_proto = ui_root.clone();

    let builder = WebViewBuilder::new()
        .with_custom_protocol("mcpguard".into(), move |_id, request| {
            serve_ui_asset(&ui_root_proto, &request)
        })
        .with_url(dashboard_entry_url())
        .with_ipc_handler(move |req| {
            let body = req.body().to_string();
            let parsed = serde_json::from_str::<serde_json::Value>(&body).ok();
            let action = parsed
                .as_ref()
                .and_then(|v| v.get("action").and_then(|a| a.as_str()))
                .unwrap_or("")
                .to_string();

            match action.as_str() {
                "window-minimize" => {
                    let _ = proxy_ipc.send_event(UserEvent::WindowMinimize);
                }
                "window-maximize" => {
                    let _ = proxy_ipc.send_event(UserEvent::WindowToggleMaximize);
                }
                "window-close" => {
                    let _ = proxy_ipc.send_event(UserEvent::WindowClose);
                }
                "scan" => {
                    spawn_dashboard_scan(
                        Arc::clone(&hooks_ipc),
                        proxy_ipc.clone(),
                        Arc::clone(&scanning_ipc),
                    );
                }
                "open-audit" => {
                    if let Err(err) = open_audit(&hooks_ipc.audit_path) {
                        crate::ui_shell::notify(
                            &hooks_ipc.catalog.toast.audit_fail_title,
                            &err.to_string(),
                        );
                    }
                    let _ = proxy_ipc.send_event(UserEvent::Refresh);
                }
                "mute" => {
                    if let Ok(mut g) = hooks_ipc.mute_until.lock() {
                        *g = Some(crate::ui_shell::mute_until_one_hour_from(SystemTime::now()));
                    }
                    crate::ui_shell::notify(
                        &hooks_ipc.catalog.toast.mute_title,
                        &hooks_ipc.catalog.toast.mute_body,
                    );
                    let _ = proxy_ipc.send_event(UserEvent::Refresh);
                }
                "vault-list" => {
                    let _ = proxy_ipc.send_event(UserEvent::Refresh);
                }
                "vault-put" => {
                    let name = parsed
                        .as_ref()
                        .and_then(|v| v.get("name").and_then(|n| n.as_str()))
                        .unwrap_or("")
                        .to_string();
                    let value = parsed
                        .as_ref()
                        .and_then(|v| v.get("value").and_then(|n| n.as_str()))
                        .unwrap_or("")
                        .to_string();
                    let hooks = Arc::clone(&hooks_ipc);
                    let proxy = proxy_ipc.clone();
                    std::thread::spawn(move || {
                        match hooks.vault.put(&name, &value) {
                            Ok(()) => {
                                crate::ui_shell::notify(
                                    &hooks.catalog.toast.vault_title,
                                    &fmt_named(
                                        &hooks.catalog.toast.vault_saved,
                                        &[("name", &name)],
                                    ),
                                );
                            }
                            Err(err) => {
                                crate::ui_shell::notify(
                                    &hooks.catalog.toast.vault_save_fail_title,
                                    &err.to_string(),
                                );
                            }
                        }
                        let _ = proxy.send_event(UserEvent::Refresh);
                    });
                }
                "allow-process" => {
                    let app = parsed
                        .as_ref()
                        .and_then(|v| v.get("app").and_then(|n| n.as_str()))
                        .unwrap_or("")
                        .to_string();
                    if app.is_empty() {
                        return;
                    }
                    let hooks = Arc::clone(&hooks_ipc);
                    let proxy = proxy_ipc.clone();
                    let scanning = Arc::clone(&scanning_ipc);
                    std::thread::spawn(move || {
                        match (hooks.allow_process)(&app) {
                            Ok(token) => {
                                crate::ui_shell::notify(
                                    &hooks.catalog.toast.allow_title,
                                    &fmt_named(
                                        &hooks.catalog.toast.allow_saved,
                                        &[("app", &token)],
                                    ),
                                );
                                spawn_dashboard_scan(hooks, proxy.clone(), scanning);
                            }
                            Err(err) => {
                                crate::ui_shell::notify(
                                    &hooks.catalog.toast.allow_fail_title,
                                    &err.to_string(),
                                );
                                let _ = proxy.send_event(UserEvent::Refresh);
                            }
                        }
                    });
                }
                "vault-delete" => {
                    let name = parsed
                        .as_ref()
                        .and_then(|v| v.get("name").and_then(|n| n.as_str()))
                        .unwrap_or("")
                        .to_string();
                    let hooks = Arc::clone(&hooks_ipc);
                    let proxy = proxy_ipc.clone();
                    std::thread::spawn(move || {
                        match hooks.vault.delete(&name) {
                            Ok(true) => {
                                crate::ui_shell::notify(
                                    &hooks.catalog.toast.vault_title,
                                    &fmt_named(
                                        &hooks.catalog.toast.vault_deleted,
                                        &[("name", &name)],
                                    ),
                                );
                            }
                            Ok(false) => {
                                crate::ui_shell::notify(
                                    &hooks.catalog.toast.vault_title,
                                    &fmt_named(
                                        &hooks.catalog.toast.vault_missing,
                                        &[("name", &name)],
                                    ),
                                );
                            }
                            Err(err) => {
                                crate::ui_shell::notify(
                                    &hooks.catalog.toast.vault_delete_fail_title,
                                    &err.to_string(),
                                );
                            }
                        }
                        let _ = proxy.send_event(UserEvent::Refresh);
                    });
                }
                _ => {}
            }
        });

    let webview = builder
        .build(&window)
        .context("create dashboard webview")?;

    let hooks_loop = Arc::clone(&hooks);
    let hide_to_tray = hooks.hide_to_tray;
    let show_slot = Arc::clone(&hooks.show_handle);
    // Register restore handle before the loop so tray can wake a hidden window.
    if let Ok(mut g) = show_slot.lock() {
        *g = Some(DashboardShowHandle {
            proxy: proxy.clone(),
        });
    }

    event_loop.run_return(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::NewEvents(StartCause::Init) => {
                refresh_dashboard_ui(&webview, &hooks_loop, &scanning);
                spawn_dashboard_scan(Arc::clone(&hooks_loop), proxy.clone(), Arc::clone(&scanning));
                let _ = webview.evaluate_script(
                    "window.mcpGuardScanning && window.mcpGuardScanning(true);",
                );
            }
            Event::UserEvent(UserEvent::Refresh) => {
                refresh_dashboard_ui(&webview, &hooks_loop, &scanning);
            }
            Event::UserEvent(UserEvent::ScanDone(summary)) => {
                let _ = webview.evaluate_script(&scan_result_js(&hooks_loop.catalog, &summary));
                let _ = webview.evaluate_script(
                    "window.mcpGuardScanning && window.mcpGuardScanning(false);",
                );
                let muted = hooks_loop
                    .mute_until
                    .lock()
                    .ok()
                    .map(|g| crate::ui_shell::is_muted(SystemTime::now(), *g))
                    .unwrap_or(false);
                // Prefer this tick for chrome — don't let stale lifetime/history fight the panel.
                let last_scan_at = (hooks_loop.status)()
                    .ok()
                    .and_then(|(s, _)| s.last_scan_at);
                let snap = AlertSnapshot {
                    exposure_count: summary.exposures,
                    activity_count: summary.activity_alerts,
                    last_scan_at,
                };
                let script =
                    snapshot_js(&snap, muted, &hooks_loop.audit_path, &hooks_loop.catalog);
                let _ = webview.evaluate_script(&script);
                if let Ok(risks) = (hooks_loop.risks)() {
                    if let Some(js) =
                        risks_from_audit_js(&hooks_loop.catalog, &snap, &risks)
                    {
                        let _ = webview.evaluate_script(&js);
                    }
                }
            }
            Event::UserEvent(UserEvent::ScanFailed(err)) => {
                let _ = webview.evaluate_script(&scan_failed_js(&hooks_loop.catalog, &err));
                let _ = webview.evaluate_script(
                    "window.mcpGuardScanning && window.mcpGuardScanning(false);",
                );
            }
            Event::UserEvent(UserEvent::WindowMinimize) => {
                if hide_to_tray {
                    window.set_visible(false);
                } else {
                    window.set_minimized(true);
                }
            }
            Event::UserEvent(UserEvent::WindowToggleMaximize) => {
                window.set_maximized(!window.is_maximized());
            }
            Event::UserEvent(UserEvent::WindowClose) => {
                if hide_to_tray {
                    window.set_visible(false);
                } else {
                    *control_flow = ControlFlow::Exit;
                }
            }
            Event::UserEvent(UserEvent::ShowWindow) => {
                window.set_minimized(false);
                window.set_visible(true);
                window.set_focus();
            }
            Event::UserEvent(UserEvent::ExitLoop) => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                if hide_to_tray {
                    window.set_visible(false);
                } else {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });

    if let Ok(mut g) = show_slot.lock() {
        *g = None;
    }
    Ok(())
}

fn spawn_dashboard_scan(
    hooks: Arc<DashboardHooks>,
    proxy: tao::event_loop::EventLoopProxy<UserEvent>,
    scanning: Arc<AtomicBool>,
) {
    if scanning.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        let result = (hooks.scan)();
        scanning.store(false, Ordering::SeqCst);
        match result {
            Ok(summary) => {
                notify_scan_finished(
                    &hooks.catalog,
                    summary.open_services,
                    summary.exposures,
                    summary.activity_alerts,
                );
                let _ = proxy.send_event(UserEvent::ScanDone(summary));
            }
            Err(err) => {
                crate::ui_shell::notify(&hooks.catalog.toast.scan_fail_title, &err.to_string());
                let _ = proxy.send_event(UserEvent::ScanFailed(err.to_string()));
            }
        }
    });
}

fn refresh_dashboard_ui(
    webview: &wry::WebView,
    hooks: &DashboardHooks,
    scanning: &AtomicBool,
) {
    let muted = hooks
        .mute_until
        .lock()
        .ok()
        .map(|g| crate::ui_shell::is_muted(SystemTime::now(), *g))
        .unwrap_or(false);
    if let Ok((snap, _)) = (hooks.status)() {
        let _ = webview.evaluate_script(&i18n_js(&hooks.catalog));
        let script = snapshot_js(&snap, muted, &hooks.audit_path, &hooks.catalog);
        let _ = webview.evaluate_script(&script);
        if scanning.load(Ordering::SeqCst) {
            let _ = webview.evaluate_script(
                "window.mcpGuardScanning && window.mcpGuardScanning(true);",
            );
        } else {
            let _ = webview.evaluate_script(
                "window.mcpGuardScanning && window.mcpGuardScanning(false);",
            );
        }
        if let Ok(risks) = (hooks.risks)() {
            if let Some(js) = risks_from_audit_js(&hooks.catalog, &snap, &risks) {
                let _ = webview.evaluate_script(&js);
            }
        }
    }
    match hooks.vault.list() {
        Ok(items) => {
            let _ = webview.evaluate_script(&vault_list_js(&items));
        }
        Err(err) => {
            let msg = json!(err.to_string());
            let _ = webview.evaluate_script(&format!(
                "window.mcpGuardResult && window.mcpGuardResult('error', {});",
                msg
            ));
        }
    }
}
