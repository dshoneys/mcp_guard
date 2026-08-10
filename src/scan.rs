//! Loopback MCP-like surface scanner.

use crate::config::Config;
use crate::contracts::Scanner;
use anyhow::Result;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

pub use crate::contracts::{HttpProbe, PortFinding, ScanReport};

/// Default Scanner adapter (plugin → contracts).
#[derive(Debug, Default, Clone, Copy)]
pub struct LoopbackScanner;

impl Scanner for LoopbackScanner {
    async fn scan(&self, cfg: &Config, extra_ports: &[u16]) -> Result<ScanReport> {
        run(cfg, extra_ports).await
    }
}

pub async fn run(cfg: &Config, extra_ports: &[u16]) -> Result<ScanReport> {
    let mut ports = cfg.scan.ports.clone();
    for p in extra_ports {
        if !ports.contains(p) {
            ports.push(*p);
        }
    }
    ports.sort_unstable();
    ports.dedup();

    let mut findings = Vec::with_capacity(ports.len());
    for port in ports {
        findings.push(probe_port(cfg, port).await);
    }

    Ok(ScanReport {
        host: cfg.scan.host.clone(),
        scanned_at: chrono::Utc::now().to_rfc3339(),
        findings,
    })
}

async fn probe_port(cfg: &Config, port: u16) -> PortFinding {
    let addr = format!("{}:{}", cfg.scan.host, port);
    let connect_to = Duration::from_millis(cfg.scan.connect_timeout_ms);

    let open = match timeout(connect_to, TcpStream::connect(&addr)).await {
        Ok(Ok(_)) => true,
        _ => false,
    };

    if !open {
        return PortFinding {
            port,
            open: false,
            http: None,
            risk_flags: vec![],
        };
    }

    let http = http_head_probe(cfg, port).await;
    let risk_flags = classify_risks(port, http.as_ref());

    PortFinding {
        port,
        open: true,
        http,
        risk_flags,
    }
}

/// Pure risk classification from an HTTP probe (unit-testable).
pub fn classify_risks(port: u16, http: Option<&HttpProbe>) -> Vec<&'static str> {
    let mut risk_flags = Vec::new();
    if let Some(h) = http {
        let acao = h
            .access_control_allow_origin
            .as_deref()
            .unwrap_or("")
            .trim();
        if acao == "*" {
            risk_flags.push("cors_star");
        }
        if h.www_authenticate.is_none()
            && (h.status_line.contains("200")
                || h.status_line.contains("404")
                || h.status_line.contains("405"))
        {
            // Heuristic only: reachable HTTP without WWW-Authenticate on first response.
            risk_flags.push("no_www_authenticate_hint");
        }
        if port == 50551 {
            risk_flags.push("known_workbuddy_ardot_port");
        }
    } else {
        risk_flags.push("tcp_open_non_http_or_timeout");
    }
    risk_flags
}

async fn http_head_probe(cfg: &Config, port: u16) -> Option<HttpProbe> {
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
