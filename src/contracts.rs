//! Stable ports + DTOs between compose (runtime/cli) and plugins.
//!
//! This module must not depend on concrete plugins. Plugins implement the traits
//! and may re-export DTOs for convenience.

use crate::config::{AuditConfig, Config};
use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

// --- Scan DTOs ---

#[derive(Debug, Serialize)]
pub struct ScanReport {
    pub host: String,
    pub scanned_at: String,
    pub findings: Vec<PortFinding>,
}

#[derive(Debug, Serialize)]
pub struct PortFinding {
    pub port: u16,
    pub open: bool,
    pub http: Option<HttpProbe>,
    pub risk_flags: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
pub struct HttpProbe {
    pub status_line: String,
    pub server: Option<String>,
    pub access_control_allow_origin: Option<String>,
    pub www_authenticate: Option<String>,
    pub body_snippet: String,
}

// --- Watch DTOs ---

#[derive(Debug, Clone, Serialize)]
pub struct PeerProcess {
    pub pid: u32,
    pub name: String,
    pub exe: Option<String>,
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortWatch {
    pub port: u16,
    /// Process(es) listening on this port (the MCP server side).
    pub listeners: Vec<PeerProcess>,
    /// Established connections touching this port on loopback.
    pub peers: Vec<ConnectionPeer>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionPeer {
    pub local: String,
    pub remote: String,
    pub state: String,
    pub processes: Vec<PeerProcess>,
    pub unknown_client: bool,
}

#[derive(Debug, Serialize)]
pub struct WatchReport {
    pub watched_at: String,
    pub ports: Vec<PortWatch>,
    pub alert_count: usize,
}

// --- Ports ---

/// Loopback / MCP-like surface probe.
pub trait Scanner: Send + Sync {
    fn scan(
        &self,
        cfg: &Config,
        extra_ports: &[u16],
    ) -> impl std::future::Future<Output = Result<ScanReport>> + Send;
}

/// Soft attribution of listeners/clients on watched ports.
pub trait Watcher: Send + Sync {
    fn watch(&self, cfg: &Config) -> Result<WatchReport>;
}

/// Append-only alert / event sink (JSONL in the default impl).
pub trait AlertSink: Send + Sync {
    fn append(&self, cfg: &AuditConfig, kind: &str, detail: Value) -> Result<()>;
}
