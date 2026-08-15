//! Loopback MCP-like surface scanner (enumerate + HTTP warn + MCP tools probe).

use crate::config::Config;
use crate::contracts::{McpProbe, Scanner};
use crate::net_enum::resolve_probe_ports;
use anyhow::Result;
use serde_json::Value;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::timeout;
use tracing::warn;

pub use crate::contracts::{HttpProbe, PortFinding, ScanReport};

const MCP_PATHS: &[&str] = &["/api/v1/mcp", "/mcp", "/", "/message"];

/// Default Scanner adapter (plugin → contracts).
#[derive(Debug, Default, Clone, Copy)]
pub struct LoopbackScanner;

impl Scanner for LoopbackScanner {
    async fn scan(&self, cfg: &Config, extra_ports: &[u16]) -> Result<ScanReport> {
        run(cfg, extra_ports).await
    }
}

pub async fn run(cfg: &Config, extra_ports: &[u16]) -> Result<ScanReport> {
    let ports = resolve_probe_ports(cfg, extra_ports);
    let mut set = JoinSet::new();
    for port in ports {
        let cfg = cfg.clone();
        set.spawn(async move { probe_port(&cfg, port).await });
    }

    let mut findings = Vec::new();
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(finding) => findings.push(finding),
            Err(err) => warn!(error = %err, "probe task join failed"),
        }
    }
    findings.sort_by_key(|f| {
        let score = f
            .risk_flags
            .iter()
            .map(|flag| match *flag {
                "mcp_tools_exposed" => 0u8,
                "xss_reflected_unescaped" => 0,
                "cors_star" => 1,
                "mcp_jsonrpc_surface" => 2,
                _ => 4,
            })
            .min()
            .unwrap_or(9);
        (score, f.port)
    });

    Ok(ScanReport {
        host: cfg.scan.host.clone(),
        scanned_at: chrono::Utc::now().to_rfc3339(),
        findings,
    })
}

async fn probe_port(cfg: &Config, port: u16) -> PortFinding {
    let addr = format!("{}:{}", cfg.scan.host, port);
    let connect_to = Duration::from_millis(cfg.scan.connect_timeout_ms);

    let open = matches!(timeout(connect_to, TcpStream::connect(&addr)).await, Ok(Ok(_)));

    if !open {
        return PortFinding {
            port,
            open: false,
            http: None,
            mcp: None,
            xss: None,
            risk_flags: vec![],
        };
    }

    let http = http_get_probe(cfg, port).await;
    // MCP endpoints may ignore GET / — always try tools/list on open TCP.
    let mcp = mcp_tools_probe(cfg, port).await;
    let xss = crate::xss_reflect::probe_port(cfg, port).await;
    let mut risk_flags = classify_risks(port, http.as_ref(), mcp.as_ref());
    if let Some(flag) = crate::xss_reflect::risk_flag_for(xss.as_ref()) {
        risk_flags.push(flag);
    }

    PortFinding {
        port,
        open: true,
        http,
        mcp,
        xss,
        risk_flags,
    }
}

pub fn is_http_status_line(status_line: &str) -> bool {
    let s = status_line.trim_start();
    s.starts_with("HTTP/1.") || s.starts_with("HTTP/2")
}

/// Pure risk classification (unit-testable).
///
/// Only **unprotected MCP** surfaces raise flags. Plain HTTP / CORS alone does not.
///
/// - MCP `tools/list` with tools → `mcp_tools_exposed` (further risk)
/// - MCP JSON-RPC shape without tools → `mcp_jsonrpc_surface` (warning)
/// - When MCP confirmed: optional `cors_star` / `no_www_authenticate_hint` / WorkBuddy pin
pub fn classify_risks(
    port: u16,
    http: Option<&HttpProbe>,
    mcp: Option<&McpProbe>,
) -> Vec<&'static str> {
    let Some(m) = mcp else {
        return vec![];
    };

    let mut risk_flags = Vec::new();
    if m.tool_count > 0 {
        risk_flags.push("mcp_tools_exposed");
    } else {
        risk_flags.push("mcp_jsonrpc_surface");
    }

    if let Some(h) = http.filter(|h| is_http_status_line(&h.status_line)) {
        let acao = h
            .access_control_allow_origin
            .as_deref()
            .unwrap_or("")
            .trim();
        if acao == "*" {
            risk_flags.push("cors_star");
        }
        if h.www_authenticate.is_none() {
            risk_flags.push("no_www_authenticate_hint");
        }
    } else {
        // MCP answered tools/list without a usable GET fingerprint — still unauthenticated surface.
        risk_flags.push("no_www_authenticate_hint");
    }

    if port == 50551 {
        risk_flags.push("known_workbuddy_ardot_port");
    }

    risk_flags
}

async fn http_get_probe(cfg: &Config, port: u16) -> Option<HttpProbe> {
    let addr = format!("{}:{}", cfg.scan.host, port);
    let http_to = Duration::from_millis(cfg.scan.http_timeout_ms);

    let result = timeout(http_to, async {
        let mut stream = TcpStream::connect(&addr).await.ok()?;
        let req = format!(
            "GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: mcp-guard/0.1\r\n\r\n",
            cfg.scan.host
        );
        stream.write_all(req.as_bytes()).await.ok()?;

        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await.ok()?;
        if n == 0 {
            return None;
        }
        let text = String::from_utf8_lossy(&buf[..n]);
        Some(parse_http_response(&text))
    })
    .await;

    match result {
        Ok(v) => v,
        Err(_) => None,
    }
}

async fn mcp_tools_probe(cfg: &Config, port: u16) -> Option<McpProbe> {
    for path in MCP_PATHS {
        if let Some(probe) = try_tools_list(cfg, port, path).await {
            return Some(probe);
        }
    }
    None
}

async fn try_tools_list(cfg: &Config, port: u16, path: &str) -> Option<McpProbe> {
    let addr = format!("{}:{}", cfg.scan.host, port);
    let http_to = Duration::from_millis(cfg.scan.http_timeout_ms.max(600));
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    })
    .to_string();

    let result = timeout(http_to, async {
        let mut stream = TcpStream::connect(&addr).await.ok()?;
        let req = format!(
            "POST {path} HTTP/1.1\r\n\
Host: {}\r\n\
Content-Type: application/json\r\n\
Accept: application/json, text/event-stream\r\n\
MCP-Protocol-Version: 2025-03-26\r\n\
Connection: close\r\n\
Content-Length: {}\r\n\
User-Agent: mcp-guard/0.1\r\n\
\r\n\
{body}",
            cfg.scan.host,
            body.len(),
        );
        stream.write_all(req.as_bytes()).await.ok()?;

        let mut buf = vec![0u8; 65536];
        let mut n = 0usize;
        loop {
            match stream.read(&mut buf[n..]).await {
                Ok(0) => break,
                Ok(k) => {
                    n += k;
                    if n >= buf.len() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        if n == 0 {
            return None;
        }
        let text = String::from_utf8_lossy(&buf[..n]);
        let http = parse_http_response(&text);
        if !is_http_status_line(&http.status_line) {
            return None;
        }
        // Auth challenge / forbidden → protected; do not score as MCP risk.
        if http.status_line.contains("401") || http.status_line.contains("403") {
            return None;
        }
        let body = extract_http_body(&text);
        classify_mcp_body(path, &body)
    })
    .await;

    match result {
        Ok(v) => v,
        Err(_) => None,
    }
}

fn extract_http_body(raw: &str) -> String {
    if let Some(idx) = raw.find("\r\n\r\n") {
        raw[idx + 4..].to_string()
    } else if let Some(idx) = raw.find("\n\n") {
        raw[idx + 2..].to_string()
    } else {
        String::new()
    }
}

/// Parse `tools/list` JSON or SSE `data:` payload.
pub fn parse_mcp_tools_payload(body: &str) -> Option<(usize, Vec<String>)> {
    let json_text = extract_json_payload(body)?;
    let v: Value = serde_json::from_str(json_text).ok()?;
    let tools = v.pointer("/result/tools")?.as_array()?;
    let sample: Vec<String> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
        .take(8)
        .collect();
    Some((tools.len(), sample))
}

fn looks_jsonrpc_mcp(body: &str) -> bool {
    let json_text = match extract_json_payload(body) {
        Some(t) => t,
        None => return false,
    };
    let Ok(v) = serde_json::from_str::<Value>(json_text) else {
        return false;
    };
    if v.get("jsonrpc").and_then(|j| j.as_str()) != Some("2.0") {
        return false;
    }
    // result or error from an MCP-ish server
    v.get("result").is_some()
        || v.get("error").is_some()
        || body.to_ascii_lowercase().contains("mcp")
}

fn classify_mcp_body(path: &str, body: &str) -> Option<McpProbe> {
    if let Some((count, sample)) = parse_mcp_tools_payload(body) {
        return Some(McpProbe {
            endpoint: path.to_string(),
            tool_count: count,
            sample_tools: sample,
        });
    }
    if looks_jsonrpc_mcp(body) {
        return Some(McpProbe {
            endpoint: path.to_string(),
            tool_count: 0,
            sample_tools: vec![],
        });
    }
    None
}

fn extract_json_payload(body: &str) -> Option<&str> {
    let trimmed = body.trim();
    if trimmed.starts_with('{') {
        return Some(trimmed);
    }
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("data:") {
            let rest = rest.trim();
            if rest.starts_with('{') {
                return Some(rest);
            }
        }
    }
    // Chunked / noisy: find first `{`
    trimmed.find('{').map(|i| &trimmed[i..])
}

pub fn parse_http_response(text: &str) -> HttpProbe {
    let mut lines = text.split("\r\n");
    let status_line = lines.next().unwrap_or("").to_string();

    let mut server = None;
    let mut acao = None;
    let mut www_authenticate = None;

    for line in lines.by_ref() {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_string();
            match name.as_str() {
                "server" => server = Some(value),
                "access-control-allow-origin" => acao = Some(value),
                "www-authenticate" => www_authenticate = Some(value),
                _ => {}
            }
        }
    }

    let body = lines.collect::<Vec<_>>().join("\n");
    let body_snippet: String = body.chars().take(240).collect();

    HttpProbe {
        status_line,
        server,
        access_control_allow_origin: acao,
        www_authenticate,
        body_snippet,
    }
}
