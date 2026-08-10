//! Runtime configuration for MCP Guard.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub scan: ScanConfig,
    pub audit: AuditConfig,
    pub serve: ServeConfig,
    pub gate: GateConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan: ScanConfig::default(),
            audit: AuditConfig::default(),
            serve: ServeConfig::default(),
            gate: GateConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScanConfig {
    /// Loopback host to probe.
    pub host: String,
    /// Default ports (WorkBuddy Ardot MCP uses 50551; Connector-like 52412).
    pub ports: Vec<u16>,
    /// TCP connect timeout in milliseconds.
    pub connect_timeout_ms: u64,
    /// HTTP read timeout in milliseconds.
    pub http_timeout_ms: u64,
    /// If true, risky exposure findings also raise alerts (not only audit rows).
    pub alert_on_exposure: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            ports: vec![50551, 52412, 3000, 8080],
            connect_timeout_ms: 400,
            http_timeout_ms: 800,
            alert_on_exposure: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditConfig {
    /// JSONL audit log path.
    pub path: PathBuf,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("mcp-guard-audit.jsonl"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServeConfig {
    /// Seconds between rescans / connection watches while serving.
    pub interval_secs: u64,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self { interval_secs: 30 }
    }
}

/// Soft gate: classify peers talking to watched ports (block comes later).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GateConfig {
    /// Process name substrings (case-insensitive) treated as trusted listeners/clients.
    /// Example: ["WorkBuddy", "CodeBuddy", "mcp-guard"]
    pub allow_process_names: Vec<String>,
    /// If true, unknown clients are audited as `gate_alert` (still not blocked in MVP).
    pub alert_on_unknown: bool,
}

impl Default for GateConfig {
    fn default() -> Self {
        Self {
            allow_process_names: vec![
                "mcp-guard".into(),
                "WorkBuddy".into(),
                "CodeBuddy".into(),
                "node".into(),
            ],
            alert_on_unknown: true,
        }
    }
}

pub fn load(explicit: Option<&Path>) -> Result<Config> {
    let path = explicit
        .map(PathBuf::from)
        .or_else(|| {
            let p = PathBuf::from("mcp-guard.toml");
            p.exists().then_some(p)
        });

    match path {
        None => Ok(Config::default()),
        Some(p) => {
            let raw = std::fs::read_to_string(&p)
                .with_context(|| format!("read config {}", p.display()))?;
            let cfg: Config = toml::from_str(&raw)
                .with_context(|| format!("parse config {}", p.display()))?;
            Ok(cfg)
        }
    }
}
