//! JSONL audit trail.

use crate::config::AuditConfig;
use crate::contracts::{AlertSink, AlertSnapshot, StatusSource};
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

/// Count alert rows and remember the latest `scan` timestamp.
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
        match kind {
            "exposure_alert" => exposure_count += 1,
            "activity_alert" => activity_count += 1,
            "scan" => {
                if let Some(t) = ts {
                    last_scan_at = Some(t);
                }
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
