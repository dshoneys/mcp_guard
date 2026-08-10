//! Resident agent loop: periodic scan + connection watch + audit.
//! Hard process gate (drop packets) is not in MVP — attribution first.
//!
//! Dispatch goes through [`crate::contracts`] only — concrete plugins are wired by CLI.

use crate::config::Config;
use crate::contracts::{AlertSink, ScanReport, Scanner, WatchReport, Watcher};
use anyhow::Result;
use tracing::{info, warn};

pub async fn run_with<S: Scanner, W: Watcher, A: AlertSink>(
    cfg: &Config,
    once: bool,
    scanner: &S,
    watcher: &W,
    sink: &A,
) -> Result<()> {
    info!(
        interval_secs = cfg.serve.interval_secs,
        audit = %cfg.audit.path.display(),
        "mcp-guard serve starting (scan + soft watch; hard gate TBD)"
    );

    loop {
        match scanner.scan(cfg, &[]).await {
            Ok(report) => {
                let open = open_services_summary(&report);
                let exposures = collect_exposures(&report);

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
                    sink.append(
                        &cfg.audit,
                        "exposure_alert",
                        serde_json::json!({
                            "host": report.host,
                            "exposures": exposures,
                            "message": "local MCP-like surface looks exploitable (config/CORS/auth heuristic)",
                        }),
                    )?;
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

        match watcher.watch(cfg) {
            Ok(report) => {
                sink.append(
                    &cfg.audit,
                    "watch",
                    serde_json::json!({
                        "alert_count": report.alert_count,
                        "ports": report.ports,
                    }),
                )?;
                if should_raise_activity_alert(&report) {
                    sink.append(
                        &cfg.audit,
                        "activity_alert",
                        serde_json::json!({
                            "alert_count": report.alert_count,
                            "message": "unknown process touching watched MCP ports",
                            "ports": report.ports.iter().filter(|p| {
                                p.peers.iter().any(|c| c.unknown_client)
                            }).collect::<Vec<_>>(),
                        }),
                    )?;
                    warn!(
                        alerts = report.alert_count,
                        "ACTIVITY ALERT: unknown process touching watched MCP ports"
                    );
                } else {
                    info!(alerts = 0, "watch complete");
                }
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

        if once {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(cfg.serve.interval_secs)).await;
    }

    Ok(())
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

pub fn should_raise_exposure_alert(report: &ScanReport, alert_on_exposure: bool) -> bool {
    alert_on_exposure && report.findings.iter().any(|f| !f.risk_flags.is_empty())
}

pub fn should_raise_activity_alert(report: &WatchReport) -> bool {
    report.alert_count > 0
}
