//! JSONL audit trail.

use crate::config::AuditConfig;
use crate::contracts::AlertSink;
use anyhow::{Context, Result};
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;

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
