//! Resident agent loop: periodic scan + connection watch + audit.
//! Hard process gate (drop packets) is not in MVP — attribution first.
//!
//! Dispatch goes through [`crate::contracts`] only — concrete plugins are wired by CLI.

use crate::config::Config;
use crate::contracts::{
    AlertSink, RiskDetail, RiskKind, ScanReport, Scanner, TickSummary, WatchReport, Watcher,
};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{info, warn};

pub async fn run_with<S: Scanner, W: Watcher, A: AlertSink>(
    cfg: &Config,
    once: bool,
    scanner: &S,
    watcher: &W,
    sink: &A,
) -> Result<()> {
    run_with_cancel(cfg, once, scanner, watcher, sink, None).await
}

/// Same as [`run_with`], but stops when `cancel` is set (e.g. tray Quit).
pub async fn run_with_cancel<S: Scanner, W: Watcher, A: AlertSink>(
    cfg: &Config,
    once: bool,
    scanner: &S,
    watcher: &W,
    sink: &A,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<()> {
    info!(
        interval_secs = cfg.serve.interval_secs,
        audit = %cfg.audit.path.display(),
        "mcp-guard serve starting (scan + soft watch; hard gate TBD)"
    );

    loop {
        if cancel.as_ref().is_some_and(|c| c.load(Ordering::SeqCst)) {
            info!("serve stopped (cancel)");
            break;
        }

        tick_once(cfg, scanner, watcher, sink).await?;

        if once {
            break;
        }

        let interval = std::time::Duration::from_secs(cfg.serve.interval_secs.max(1));
        let mut slept = std::time::Duration::ZERO;
        let step = std::time::Duration::from_millis(200);
        while slept < interval {
            if cancel.as_ref().is_some_and(|c| c.load(Ordering::SeqCst)) {
                info!("serve stopped (cancel during wait)");
                return Ok(());
            }
            let chunk = step.min(interval - slept);
            tokio::time::sleep(chunk).await;
            slept += chunk;
        }
    }

    Ok(())
}

/// One scan + watch + audit cycle (used by serve loop and tests).
pub async fn tick_once<S: Scanner, W: Watcher, A: AlertSink>(
    cfg: &Config,
    scanner: &S,
    watcher: &W,
    sink: &A,
) -> Result<TickSummary> {
    let mut summary = TickSummary::default();

    let mut scan_report: Option<ScanReport> = None;

    match scanner.scan(cfg, &[]).await {
        Ok(report) => {
            let open = open_services_summary(&report);
            let exposures = collect_exposures(&report);
            summary.open_services = open.len();
            summary.exposures = exposures.len();

            sink.append(
                &cfg.audit,
                "scan",
                serde_json::json!({
                    "host": report.host,
                    "open_services": open,
                    "exposure_count": exposures.len(),
                }),
            )?;

            if should_raise_exposure_alert(&report, cfg.scan.alert_on_exposure) {
                warn!(
                    count = exposures.len(),
                    host = %report.host,
                    "EXPOSURE ALERT: exploitable local MCP surface detected"
                );
            } else {
                info!(
                    open = open.len(),
                    exposures = exposures.len(),
                    "scan complete"
                );
            }
            scan_report = Some(report);
        }
        Err(err) => {
            warn!(error = %err, "scan failed");
            sink.append(
                &cfg.audit,
                "scan_error",
                serde_json::json!({ "error": err.to_string() }),
            )?;
        }
    }

    let mut watch_report: Option<WatchReport> = None;
    match watcher.watch(cfg) {
        Ok(report) => {
            summary.activity_alerts = report.alert_count;
            sink.append(
                &cfg.audit,
                "watch",
                serde_json::json!({
                    "alert_count": report.alert_count,
                    "ports": report.ports,
                }),
            )?;
            if should_raise_activity_alert(&report) {
                warn!(
                    alerts = report.alert_count,
                    "ACTIVITY ALERT: unknown process touching watched MCP ports"
                );
            } else {
                info!(alerts = 0, "watch complete");
            }
            watch_report = Some(report);
        }
        Err(err) => {
            warn!(error = %err, "watch failed");
            sink.append(
                &cfg.audit,
                "watch_error",
                serde_json::json!({ "error": err.to_string() }),
            )?;
        }
    }

    // Build human-facing risk rows after both probes so we can attach listener apps.
    if let Some(report) = &scan_report {
        let mut risks = risk_details_from_scan(report);
        if let Some(w) = &watch_report {
            attach_listener_apps(&mut risks, w);
        }
        if should_raise_exposure_alert(report, cfg.scan.alert_on_exposure) {
            sink.append(
                &cfg.audit,
                "exposure_alert",
                serde_json::json!({
                    "host": report.host,
                    "exposures": risks.iter().filter(|r| r.kind == RiskKind::Exposure).collect::<Vec<_>>(),
                    "message": "local MCP-like surface looks exploitable (config/CORS/auth heuristic)",
                }),
            )?;
        }
        summary.risks.extend(risks);
    }
    if let Some(report) = &watch_report {
        let activity = risk_details_from_watch(report);
        if should_raise_activity_alert(report) {
            sink.append(
                &cfg.audit,
                "activity_alert",
                serde_json::json!({
                    "alert_count": report.alert_count,
                    "message": "unknown process touching watched MCP ports",
                    "risks": activity,
                    "ports": report.ports.iter().filter(|p| {
                        p.peers.iter().any(|c| c.unknown_client)
                    }).collect::<Vec<_>>(),
                }),
            )?;
        }
        summary.risks.extend(activity);
    }

    Ok(summary)
}

pub fn open_services_summary(report: &ScanReport) -> Vec<serde_json::Value> {
    report
        .findings
        .iter()
        .filter(|f| f.open)
        .map(|f| {
            serde_json::json!({
                "port": f.port,
                "risk_flags": f.risk_flags,
                "acao": f.http.as_ref().and_then(|h| h.access_control_allow_origin.clone()),
            })
        })
        .collect()
}

pub fn collect_exposures(report: &ScanReport) -> Vec<serde_json::Value> {
    report
        .findings
        .iter()
        .filter(|f| !f.risk_flags.is_empty())
        .map(|f| {
            serde_json::json!({
                "port": f.port,
                "open": f.open,
                "risk_flags": f.risk_flags,
                "acao": f.http.as_ref().and_then(|h| h.access_control_allow_origin.clone()),
            })
        })
        .collect()
}

pub fn risk_details_from_scan(report: &ScanReport) -> Vec<RiskDetail> {
    report
        .findings
        .iter()
        .filter(|f| !f.risk_flags.is_empty())
        .map(|f| {
            let server = f.http.as_ref().and_then(|h| h.server.clone());
            let acao = f
                .http
                .as_ref()
                .and_then(|h| h.access_control_allow_origin.clone());
            RiskDetail {
                kind: RiskKind::Exposure,
                port: f.port,
                app: infer_app_name(f.port, server.as_deref()),
                mcp: mcp_surface_label(f.port),
                flags: f.risk_flags.iter().map(|s| (*s).to_string()).collect(),
                note: {
                    let mut parts = Vec::new();
                    if let Some(a) = acao {
                        parts.push(format!("ACAO={a}"));
                    }
                    if let Some(m) = &f.mcp {
                        if m.tool_count > 0 {
                            let tools = if m.sample_tools.is_empty() {
                                format!("{} tools", m.tool_count)
                            } else {
                                format!(
                                    "{} tools: {}",
                                    m.tool_count,
                                    m.sample_tools.join(", ")
                                )
                            };
                            parts.push(format!("{} · {tools}", m.endpoint));
                        } else {
                            parts.push(format!("{} · JSON-RPC", m.endpoint));
                        }
                    }
                    parts.join(" · ")
                },
            }
        })
        .collect()
}

pub fn risk_details_from_watch(report: &WatchReport) -> Vec<RiskDetail> {
    let mut out = Vec::new();
    for port in &report.ports {
        let listener_app = port
            .listeners
            .first()
            .map(|p| display_process(p))
            .unwrap_or_else(|| infer_app_name(port.port, None));
        for peer in port.peers.iter().filter(|p| p.unknown_client) {
            let client = peer
                .processes
                .first()
                .map(|p| display_process(p))
                .unwrap_or_else(|| "未知客户端".into());
            out.push(RiskDetail {
                kind: RiskKind::Activity,
                port: port.port,
                app: client,
                mcp: mcp_surface_label(port.port),
                flags: vec!["unknown_client".into()],
                note: format!(
                    "连入 {} → {}（监听方：{listener_app}）",
                    peer.local, peer.remote
                ),
            });
        }
    }
    out
}

/// Attach listening process names from watch onto exposure rows for the same port.
pub fn attach_listener_apps(risks: &mut [RiskDetail], watch: &WatchReport) {
    for risk in risks.iter_mut() {
        if risk.kind != RiskKind::Exposure {
            continue;
        }
        if let Some(port) = watch.ports.iter().find(|p| p.port == risk.port) {
            if let Some(listener) = port.listeners.first() {
                risk.app = display_process(listener);
            }
        }
        if risk.app.is_empty() {
            risk.app = infer_app_name(risk.port, None);
        }
        if risk.mcp.is_empty() {
            risk.mcp = mcp_surface_label(risk.port);
        }
    }
}

fn display_process(p: &crate::contracts::PeerProcess) -> String {
    if let Some(exe) = p.exe.as_ref().filter(|e| !e.is_empty()) {
        let leaf = std::path::Path::new(exe)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(exe.as_str());
        if leaf.eq_ignore_ascii_case(&p.name) || p.name.is_empty() {
            leaf.to_string()
        } else {
            format!("{} ({leaf})", p.name)
        }
    } else if !p.name.is_empty() {
        p.name.clone()
    } else {
        format!("pid {}", p.pid)
    }
}

/// Best-effort product / process label when OS attribution is missing.
pub fn infer_app_name(port: u16, server_header: Option<&str>) -> String {
    if let Some(s) = server_header.map(str::trim).filter(|s| !s.is_empty()) {
        return s.to_string();
    }
    match port {
        50551 => "WorkBuddy".into(),
        52412 => "CodeBuddy / 相关 Agent".into(),
        _ => String::new(),
    }
}

/// Which MCP / local surface this port usually is.
pub fn mcp_surface_label(port: u16) -> String {
    match port {
        50551 => "WorkBuddy ARDOT MCP".into(),
        52412 => "本机 Agent MCP 表面".into(),
        3000 | 8080 => format!("本机 HTTP 服务 :{port}"),
        _ => format!("本机 loopback MCP 表面 :{port}"),
    }
}

pub fn should_raise_exposure_alert(report: &ScanReport, alert_on_exposure: bool) -> bool {
    alert_on_exposure && report.findings.iter().any(|f| !f.risk_flags.is_empty())
}

pub fn should_raise_activity_alert(report: &WatchReport) -> bool {
    report.alert_count > 0
}
