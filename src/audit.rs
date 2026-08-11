//! JSONL audit trail.

use crate::config::AuditConfig;
use crate::contracts::{AlertSink, AlertSnapshot, RiskDetail, RiskKind, StatusSource};
use crate::serve::{infer_app_name, mcp_surface_label};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct AuditEvent<'a> {
    pub ts: String,
    pub kind: &'a str,
    pub detail: serde_json::Value,
}

/// Default AlertSink adapter (plugin → contracts).
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonlSink;

impl AlertSink for JsonlSink {
    fn append(&self, cfg: &AuditConfig, kind: &str, detail: Value) -> Result<()> {
        append(cfg, kind, detail)
    }
}

/// Default StatusSource: scan audit JSONL for alert kinds.
#[derive(Debug, Default, Clone, Copy)]
pub struct JsonlStatusSource;

impl StatusSource for JsonlStatusSource {
    fn snapshot(&self, audit_path: &Path) -> Result<AlertSnapshot> {
        snapshot_from_jsonl(audit_path)
    }
}

pub fn append(cfg: &AuditConfig, kind: &str, detail: serde_json::Value) -> Result<()> {
    if let Some(parent) = cfg.path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create audit dir {}", parent.display()))?;
        }
    }

    let event = AuditEvent {
        ts: Utc::now().to_rfc3339(),
        kind,
        detail,
    };
    let line = serde_json::to_string(&event)?;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&cfg.path)
        .with_context(|| format!("open audit log {}", cfg.path.display()))?;
    writeln!(f, "{line}")?;
    Ok(())
}

/// Current risk posture for tray/dashboard chrome (not lifetime alert totals).
///
/// Derived from the latest `scan` / `watch` rows in the audit JSONL:
/// - `exposure_count` ← last scan's `exposure_count`
/// - `activity_count` ← last watch's `alert_count`
pub fn snapshot_from_jsonl(path: &Path) -> Result<AlertSnapshot> {
    if !path.exists() {
        return Ok(AlertSnapshot::default());
    }
    let f = std::fs::File::open(path)
        .with_context(|| format!("open audit for status {}", path.display()))?;
    let reader = BufReader::new(f);

    let mut exposure_count = 0usize;
    let mut activity_count = 0usize;
    let mut last_scan_at = None;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let kind = v.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        let ts = v
            .get("ts")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string());
        let detail = v.get("detail");
        match kind {
            "scan" => {
                exposure_count = detail
                    .and_then(|d| d.get("exposure_count"))
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize;
                if let Some(t) = ts {
                    last_scan_at = Some(t);
                }
            }
            "watch" => {
                activity_count = detail
                    .and_then(|d| d.get("alert_count"))
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize;
            }
            _ => {}
        }
    }

    Ok(AlertSnapshot {
        exposure_count,
        activity_count,
        last_scan_at,
    })
}

/// Reconstruct risk lines for the UI from the latest *still-active* alert rows.
/// A later clean `scan` / `watch` clears the corresponding risk set.
pub fn latest_risks_from_jsonl(path: &Path) -> Result<Vec<RiskDetail>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let f = std::fs::File::open(path)
        .with_context(|| format!("open audit for risks {}", path.display()))?;
    let reader = BufReader::new(f);

    let mut last_exposure: Option<Value> = None;
    let mut last_activity: Option<Value> = None;
    let mut last_scan_exposures: usize = 0;
    let mut last_watch_alerts: usize = 0;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let kind = v.get("kind").and_then(|k| k.as_str()).unwrap_or("");
        let detail = v.get("detail").cloned().unwrap_or(Value::Null);
        match kind {
            "scan" => {
                last_scan_exposures = detail
                    .get("exposure_count")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize;
                if last_scan_exposures == 0 {
                    last_exposure = None;
                }
            }
            "watch" => {
                last_watch_alerts = detail
                    .get("alert_count")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize;
                if last_watch_alerts == 0 {
                    last_activity = None;
                }
            }
            "exposure_alert" => last_exposure = Some(detail),
            "activity_alert" => last_activity = Some(detail),
            _ => {}
        }
    }

    let mut risks = Vec::new();
    if last_scan_exposures > 0 {
        if let Some(detail) = last_exposure {
            if let Some(arr) = detail.get("exposures").and_then(|e| e.as_array()) {
                for item in arr {
                    if let Some(r) = parse_risk_value(item, RiskKind::Exposure) {
                        risks.push(r);
                    }
                }
            }
        }
    }
    if last_watch_alerts > 0 {
        if let Some(detail) = last_activity {
            if let Some(arr) = detail.get("risks").and_then(|e| e.as_array()) {
                for item in arr {
                    if let Some(r) = parse_risk_value(item, RiskKind::Activity) {
                        risks.push(r);
                    }
                }
            } else if let Some(ports) = detail.get("ports").and_then(|p| p.as_array()) {
                for port_obj in ports {
                    let port = port_obj.get("port").and_then(|p| p.as_u64()).unwrap_or(0) as u16;
                    let peers = port_obj
                        .get("peers")
                        .and_then(|p| p.as_array())
                        .cloned()
                        .unwrap_or_default();
                    for peer in peers.iter().filter(|p| {
                        p.get("unknown_client")
                            .and_then(|u| u.as_bool())
                            .unwrap_or(false)
                    }) {
                        let procs = peer
                            .get("processes")
                            .and_then(|p| p.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|x| x.get("name").and_then(|n| n.as_str()))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            })
                            .unwrap_or_default();
                        let local = peer.get("local").and_then(|s| s.as_str()).unwrap_or("?");
                        let remote = peer.get("remote").and_then(|s| s.as_str()).unwrap_or("?");
                        let note = if procs.is_empty() {
                            format!("{local} → {remote}")
                        } else {
                            format!("{procs} · {local} → {remote}")
                        };
                        if port > 0 {
                            risks.push(RiskDetail {
                                kind: RiskKind::Activity,
                                port,
                                app: if procs.is_empty() {
                                    String::new()
                                } else {
                                    procs
                                },
                                mcp: mcp_surface_label(port),
                                flags: vec!["unknown_client".into()],
                                note,
                            });
                        }
                    }
                }
            }
        }
    }
    Ok(risks)
}

fn parse_risk_value(item: &Value, fallback_kind: RiskKind) -> Option<RiskDetail> {
    if let Ok(mut r) = serde_json::from_value::<RiskDetail>(item.clone()) {
        if r.port == 0 {
            return None;
        }
        if r.app.is_empty() {
            r.app = infer_app_name(r.port, None);
        }
        if r.mcp.is_empty() {
            r.mcp = mcp_surface_label(r.port);
        }
        if r.flags.is_empty() && fallback_kind == RiskKind::Exposure {
            return None;
        }
        return Some(r);
    }

    let port = item.get("port").and_then(|p| p.as_u64()).unwrap_or(0) as u16;
    let mut flags: Vec<String> = item
        .get("flags")
        .or_else(|| item.get("risk_flags"))
        .and_then(|f| f.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    if flags.is_empty() && fallback_kind == RiskKind::Exposure {
        return None;
    }
    if flags.is_empty() && fallback_kind == RiskKind::Activity {
        flags.push("unknown_client".into());
    }
    let note = item
        .get("note")
        .and_then(|a| a.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            item.get("acao")
                .and_then(|a| a.as_str())
                .map(|a| format!("ACAO={a}"))
        })
        .unwrap_or_default();
    if port == 0 {
        return None;
    }
    Some(RiskDetail {
        kind: fallback_kind,
        port,
        app: item
            .get("app")
            .and_then(|a| a.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| infer_app_name(port, None)),
        mcp: item
            .get("mcp")
            .and_then(|a| a.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| mcp_surface_label(port)),
        flags,
        note,
    })
}
