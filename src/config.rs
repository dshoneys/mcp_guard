//! Runtime configuration for MCP Guard.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub scan: ScanConfig,
    pub git_scan: GitScanConfig,
    pub audit: AuditConfig,
    pub serve: ServeConfig,
    pub gate: GateConfig,
    pub vault: VaultConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan: ScanConfig::default(),
            git_scan: GitScanConfig::default(),
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
    /// Probe local HTTP for classic reflected XSS (URL canary → HTML).
    pub xss_reflect: bool,
    /// Max canary GETs per open port.
    pub xss_max_probes_per_port: usize,
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
            xss_reflect: true,
            xss_max_probes_per_port: 6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GitScanConfig {
    /// Skip files larger than this many bytes.
    pub max_file_bytes: u64,
    /// If non-empty, only scan paths ending with these suffixes (case-insensitive).
    pub extensions: Vec<String>,
    /// Skip relative paths containing any of these substrings (normalized `/`).
    pub exclude_substrings: Vec<String>,
}

impl Default for GitScanConfig {
    fn default() -> Self {
        Self {
            max_file_bytes: 5_000_000,
            extensions: vec![
                ".json".into(),
                ".jsonl".into(),
                ".ndjson".into(),
                ".txt".into(),
                ".md".into(),
                ".log".into(),
                ".yml".into(),
                ".yaml".into(),
                ".toml".into(),
                ".ts".into(),
                ".js".into(),
                ".py".into(),
            ],
            // Lab fixtures + HTML dumps that intentionally embed opaque blobs for demos.
            exclude_substrings: vec![
                "cases/arxiv-2608-09867/fixtures/".into(),
                "cases/arxiv-2608-09867/assets/".into(),
                "experiments/git-cipher-scan/".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AuditConfig {
    /// JSONL audit log path.
    pub path: PathBuf,
    /// Keep transient activity alerts visible in tray/dashboard after connections close.
    pub activity_alert_ttl_secs: u64,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("mcp-guard-audit.jsonl"),
            activity_alert_ttl_secs: 600,
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
                // IDE hosts: allowlist is secondary; same-family feature is primary.
                "Cursor".into(),
                "Code".into(),
                "Code - Insiders".into(),
                "devenv".into(),
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

    let mut cfg = match path {
        None => Config::default(),
        Some(p) => {
            let raw = std::fs::read_to_string(&p)
                .with_context(|| format!("read config {}", p.display()))?;
            toml::from_str(&raw).with_context(|| format!("parse config {}", p.display()))?
        }
    };
    merge_manual_allows(&mut cfg.gate)?;
    Ok(cfg)
}

/// JSON list of operator-approved client process tokens (merged into `[gate]`).
pub const MANUAL_ALLOWS_FILE: &str = "mcp-guard-manual-allows.json";

/// Normalize UI/process names for gate substring matching (`msedge.exe` → `msedge`).
pub fn normalize_allow_token(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    let base = s.rsplit(['\\', '/']).next().unwrap_or(s);
    base.strip_suffix(".exe")
        .or_else(|| base.strip_suffix(".EXE"))
        .unwrap_or(base)
        .to_string()
}

pub fn load_manual_allows() -> Result<Vec<String>> {
    let path = PathBuf::from(MANUAL_ALLOWS_FILE);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("read manual allows {}", path.display()))?;
    let list: Vec<String> = serde_json::from_str(&raw)
        .with_context(|| format!("parse manual allows {}", path.display()))?;
    Ok(list
        .into_iter()
        .map(|s| normalize_allow_token(&s))
        .filter(|s| !s.is_empty())
        .collect())
}

fn save_manual_allows(list: &[String]) -> Result<()> {
    let path = PathBuf::from(MANUAL_ALLOWS_FILE);
    let raw = serde_json::to_string_pretty(list)?;
    std::fs::write(&path, raw).with_context(|| format!("write manual allows {}", path.display()))?;
    Ok(())
}

pub fn merge_manual_allows(gate: &mut GateConfig) -> Result<()> {
    for name in load_manual_allows()? {
        if !gate
            .allow_process_names
            .iter()
            .any(|x| x.eq_ignore_ascii_case(&name))
        {
            gate.allow_process_names.push(name);
        }
    }
    Ok(())
}

/// Persist a manual allow and merge into the in-memory gate config.
pub fn add_manual_allow(cfg: &mut Config, process: &str) -> Result<String> {
    let token = normalize_allow_token(process);
    if token.is_empty() {
        anyhow::bail!("process name required");
    }
    let mut list = load_manual_allows()?;
    if !list.iter().any(|x| x.eq_ignore_ascii_case(&token)) {
        list.push(token.clone());
        save_manual_allows(&list)?;
    }
    merge_manual_allows(&mut cfg.gate)?;
    Ok(token)
}
