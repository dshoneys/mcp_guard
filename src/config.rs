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
    pub vault: VaultConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan: ScanConfig::default(),
            audit: AuditConfig::default(),
            serve: ServeConfig::default(),
            gate: GateConfig::default(),
            vault: VaultConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScanConfig {
    /// Loopback host used for connect/HTTP probes.
    pub host: String,
    /// When true (default), probe every TCP LISTEN on loopback/unspecified — not a fixed whitelist.
    pub discover_listeners: bool,
    /// Optional extra ports always probed (even if currently closed).
    pub ports: Vec<u16>,
    /// Cap on probe set size after discover+extras merge.
    pub max_probe_ports: usize,
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
            discover_listeners: true,
            ports: vec![],
            max_probe_ports: 512,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VaultConfig {
    /// Encrypted secrets blob.
    pub store_path: PathBuf,
    /// 32-byte key file (created on first use).
    pub key_path: PathBuf,
    /// Default TTL for issued refs (seconds).
    pub ref_ttl_secs: u64,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            store_path: PathBuf::from("mcp-guard-vault.enc"),
            key_path: PathBuf::from("mcp-guard-vault.key"),
            ref_ttl_secs: 600,
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
