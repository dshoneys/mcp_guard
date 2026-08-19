//! JSONL audit trail.

use crate::config::AuditConfig;
use crate::contracts::{AlertSink, AlertSnapshot, RiskDetail, RiskKind, StatusSource};
use crate::serve::{infer_app_name, mcp_surface_label};
use crate::watch::is_allowed;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// How long transient activity alerts stay visible in tray/dashboard after connections close.
pub const DEFAULT_ACTIVITY_ALERT_TTL_SECS: u64 = 600;

const RECENT_ACTIVITY_NOTE_SUFFIX: &str = "（连接已断开，近期告警）";

#[derive(Debug, Serialize)]
pub struct AuditEvent<'a> {
    pub ts: String,
    pub kind: &'a str,
    pub detail: serde_json::Value,
}

/// Default AlertSink adapter (plugin → contracts).
#[derive(Debug, Clone, Copy)]
pub struct JsonlSink;

impl Default for JsonlSink {
    fn default() -> Self {
        Self
    }
}

impl AlertSink for JsonlSink {
    fn append(&self, cfg: &AuditConfig, kind: &str, detail: Value) -> Result<()> {
        append(cfg, kind, detail)
    }
}

/// Default StatusSource: scan audit JSONL for alert kinds.
#[derive(Debug, Clone, Copy)]
pub struct JsonlStatusSource {
    pub activity_alert_ttl_secs: u64,
}

impl Default for JsonlStatusSource {
    fn default() -> Self {
        Self {
            activity_alert_ttl_secs: DEFAULT_ACTIVITY_ALERT_TTL_SECS,
        }
    }
}

impl JsonlStatusSource {
    pub fn new(activity_alert_ttl_secs: u64) -> Self {
        Self {
            activity_alert_ttl_secs,
        }
    }

    pub fn from_audit_cfg(cfg: &AuditConfig) -> Self {
        Self::new(cfg.activity_alert_ttl_secs)
    }
}

impl StatusSource for JsonlStatusSource {
    fn snapshot(&self, audit_path: &Path) -> Result<AlertSnapshot> {
        snapshot_from_jsonl(audit_path, self.activity_alert_ttl_secs)
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

#[derive(Debug, Default)]
struct AuditRollup {
    exposure_count: usize,
    activity_count: usize,
    last_scan_at: Option<String>,
    last_exposure: Option<Value>,
    last_activity: Option<Value>,
    last_scan_exposures: usize,
    last_watch_alerts: usize,
    activity_alerts: Vec<(DateTime<Utc>, Value)>,
}

fn parse_event_ts(ts: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn rollup_from_jsonl(path: &Path) -> Result<AuditRollup> {
    if !path.exists() {
        return Ok(AuditRollup::default());
    }
    let f = std::fs::File::open(path)
        .with_context(|| format!("open audit for rollup {}", path.display()))?;
    let reader = BufReader::new(f);

    let mut rollup = AuditRollup::default();
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
        let detail = v.get("detail").cloned().unwrap_or(Value::Null);
        match kind {
            "scan" => {
                rollup.exposure_count = detail
                    .get("exposure_count")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize;
                rollup.last_scan_exposures = rollup.exposure_count;
                if rollup.last_scan_exposures == 0 {
                    rollup.last_exposure = None;
                }
                if let Some(t) = ts {
                    rollup.last_scan_at = Some(t);
                }
            }
            "watch" => {
                rollup.activity_count = detail
                    .get("alert_count")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as usize;
                rollup.last_watch_alerts = rollup.activity_count;
                if rollup.last_watch_alerts == 0 {
                    rollup.last_activity = None;
                }
            }
            "exposure_alert" => rollup.last_exposure = Some(detail),
            "activity_alert" => {
                rollup.last_activity = Some(detail.clone());
                if let Some(ts) = ts.as_deref().and_then(parse_event_ts) {
                    rollup.activity_alerts.push((ts, detail));
                }
            }
            _ => {}
        }
    }
    Ok(rollup)
}

/// Current risk posture for tray/dashboard chrome (not lifetime alert totals).
///
/// Derived from the latest `scan` / `watch` rows in the audit JSONL:
/// - `exposure_count` ← last scan's `exposure_count`
/// - `activity_count` ← last watch's `alert_count`, or recent `activity_alert` rows within TTL
pub fn snapshot_from_jsonl(path: &Path, activity_alert_ttl_secs: u64) -> Result<AlertSnapshot> {
    let rollup = rollup_from_jsonl(path)?;
    let gate = activity_gate()?;
    let recent_activity =
        filter_allowed_activity(recent_activity_risks(&rollup.activity_alerts, activity_alert_ttl_secs), &gate);
    let activity_count = rollup.activity_count.max(recent_activity.len());
    Ok(AlertSnapshot {
        exposure_count: rollup.exposure_count,
        activity_count,
        last_scan_at: rollup.last_scan_at,
    })
}

/// Reconstruct risk lines for the UI from active alerts and recent transient activity alerts.
/// A later clean `scan` clears exposure risks; activity risks linger for `activity_alert_ttl_secs`.
pub fn latest_risks_from_jsonl(path: &Path, activity_alert_ttl_secs: u64) -> Result<Vec<RiskDetail>> {
    let rollup = rollup_from_jsonl(path)?;
    let mut risks = Vec::new();

    if rollup.last_scan_exposures > 0 {
        if let Some(detail) = rollup.last_exposure {
            if let Some(arr) = detail.get("exposures").and_then(|e| e.as_array()) {
                for item in arr {
                    if let Some(r) = parse_risk_value(item, RiskKind::Exposure) {
                        risks.push(r);
                    }
                }
            }
        }
    }

    if rollup.last_watch_alerts > 0 {
        if let Some(detail) = rollup.last_activity {
            risks.extend(activity_risks_from_detail(&detail, false));
        }
    } else {
        risks.extend(recent_activity_risks(
            &rollup.activity_alerts,
            activity_alert_ttl_secs,
        ));
    }

    let gate = activity_gate()?;
    risks = filter_allowed_activity(risks, &gate);

    Ok(risks)
}

fn recent_activity_risks(
    alerts: &[(DateTime<Utc>, Value)],
    ttl_secs: u64,
) -> Vec<RiskDetail> {
    if alerts.is_empty() || ttl_secs == 0 {
        return Vec::new();
    }
    let now = Utc::now();
    let cutoff = now - chrono::Duration::seconds(ttl_secs as i64);
    let mut by_key: HashMap<(u16, String), (DateTime<Utc>, RiskDetail)> = HashMap::new();

    for (ts, detail) in alerts {
        if *ts < cutoff {
            continue;
        }
        for risk in activity_risks_from_detail(detail, true) {
            let key = (risk.port, risk.app.clone());
            match by_key.get(&key) {
                Some((prev_ts, _)) if *prev_ts >= *ts => {}
                _ => {
                    by_key.insert(key, (*ts, risk));
                }
            }
        }
    }

    let mut out: Vec<_> = by_key.into_values().map(|(_, risk)| risk).collect();
    out.sort_by(|a, b| a.port.cmp(&b.port).then_with(|| a.app.cmp(&b.app)));
    out
}

fn activity_risks_from_detail(detail: &Value, mark_recent: bool) -> Vec<RiskDetail> {
    let mut risks = Vec::new();
    if let Some(arr) = detail.get("risks").and_then(|e| e.as_array()) {
        for item in arr {
            if let Some(mut r) = parse_risk_value(item, RiskKind::Activity) {
                if mark_recent {
                    append_recent_activity_note(&mut r);
                }
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
                    let mut risk = RiskDetail {
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
                    };
                    if mark_recent {
                        append_recent_activity_note(&mut risk);
                    }
                    risks.push(risk);
                }
            }
        }
    }
    risks
}

fn append_recent_activity_note(risk: &mut RiskDetail) {
    if !risk.note.contains(RECENT_ACTIVITY_NOTE_SUFFIX) {
        if risk.note.is_empty() {
            risk.note = RECENT_ACTIVITY_NOTE_SUFFIX.to_string();
        } else {
            risk.note.push_str(RECENT_ACTIVITY_NOTE_SUFFIX);
        }
    }
}

fn activity_gate() -> Result<crate::config::GateConfig> {
    Ok(crate::config::load(None)?.gate)
}

fn filter_allowed_activity(risks: Vec<RiskDetail>, gate: &crate::config::GateConfig) -> Vec<RiskDetail> {
    risks
        .into_iter()
        .filter(|r| {
            if r.kind != RiskKind::Activity {
                return true;
            }
            if r.app.trim().is_empty() {
                return true;
            }
            !is_allowed(&r.app, None, gate)
        })
        .collect()
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
