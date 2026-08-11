//! Stable ports + DTOs between compose (runtime/cli) and plugins.
//!
//! This module must not depend on concrete plugins. Plugins implement the traits
//! and may re-export DTOs for convenience.

use crate::config::{AuditConfig, Config};
use anyhow::Result;
use serde::{Deserialize, Serialize};
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
    /// Present when JSON-RPC `tools/list` (or MCP-shaped response) succeeded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp: Option<McpProbe>,
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

#[derive(Debug, Clone, Serialize)]
pub struct McpProbe {
    /// Path that answered (e.g. `/api/v1/mcp`).
    pub endpoint: String,
    pub tool_count: usize,
    /// Up to a few tool names for the UI note.
    pub sample_tools: Vec<String>,
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

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct TickSummary {
    pub open_services: usize,
    pub exposures: usize,
    pub activity_alerts: usize,
    /// Structured risks for UI listing (not only counts).
    #[serde(default)]
    pub risks: Vec<RiskDetail>,
}

impl TickSummary {
    pub fn has_risk(&self) -> bool {
        self.exposures > 0 || self.activity_alerts > 0 || !self.risks.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskKind {
    Exposure,
    Activity,
}

/// One actionable risk line for the dashboard scan panel.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskDetail {
    pub kind: RiskKind,
    pub port: u16,
    /// Listening / client process display name when known.
    #[serde(default)]
    pub app: String,
    /// Human MCP / surface label (e.g. WorkBuddy ARDOT).
    #[serde(default)]
    pub mcp: String,
    /// Machine flag codes; UI maps to locale descriptions.
    #[serde(default)]
    pub flags: Vec<String>,
    /// Extra technical context (ACAO value, connection ends, etc.).
    #[serde(default)]
    pub note: String,
}

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

/// Recent alert counts derived from the audit trail (for presentation).
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct AlertSnapshot {
    pub exposure_count: usize,
    pub activity_count: usize,
    pub last_scan_at: Option<String>,
}

/// Read presentation inputs without depending on scan/watch plugins.
pub trait StatusSource: Send + Sync {
    fn snapshot(&self, audit_path: &std::path::Path) -> Result<AlertSnapshot>;
}

/// Tray / chrome severity (maps to UX states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardSeverity {
    Ok,
    Warn,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrayActionId {
    OpenDashboard,
    OpenAudit,
    ScanNow,
    Mute,
    Quit,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TrayMenuItem {
    pub action: TrayActionId,
    pub label: String,
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TrayMenuModel {
    pub state_id: String,
    pub severity: GuardSeverity,
    pub header_label: String,
    pub muted: bool,
    pub items: Vec<TrayMenuItem>,
}
