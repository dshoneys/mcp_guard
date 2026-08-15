//! Reflected XSS canary probe (classic URL → HTML echo).
//!
//! Scope: unescaped reflection of request-controlled strings into `text/html`.
//! Not: intentional StaticHtml / MCP Apps script execution from disk.

use crate::config::Config;
use crate::contracts::XssReflectProbe;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Marker bytes that must survive HTML escaping to count as unescaped reflection.
pub const CANARY_MARKERS: &str = "<>\"'";

/// Outcome of inspecting one HTML body for a canary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectKind {
    /// Raw canary (incl. markers) present in body.
    Unescaped,
    /// Only HTML-escaped forms of the canary found.
    Escaped,
    /// Canary not present.
    None,
}

/// Build a unique canary: `mgx{token}<>"'`
pub fn make_canary(token: &str) -> String {
    format!("mgx{token}{CANARY_MARKERS}")
}

/// HTML-escape the full canary the way a careful error page would.
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Classify whether `body` reflects `canary` unescaped, escaped-only, or not at all.
pub fn classify_reflection(body: &str, canary: &str) -> ReflectKind {
    if body.contains(canary) {
        return ReflectKind::Unescaped;
    }
    let escaped = html_escape(canary);
    if body.contains(&escaped) {
        return ReflectKind::Escaped;
    }
    // Partial: token alone without markers is weak — do not score as XSS opportunity.
    ReflectKind::None
}

pub fn looks_like_html(content_type: Option<&str>, body: &str) -> bool {
    if let Some(ct) = content_type {
        let lower = ct.to_ascii_lowercase();
        if lower.contains("text/html") || lower.contains("application/xhtml") {
            return true;
        }
        // Explicit non-HTML → skip even if body has tags.
        if lower.contains("application/json")
            || lower.contains("text/plain")
            || lower.contains("application/javascript")
            || lower.contains("text/css")
            || lower.contains("image/")
        {
            return false;
        }
    }
    let trim = body.trim_start();
    trim.starts_with("<!DOCTYPE")
        || trim.starts_with("<!doctype")
        || trim.starts_with("<html")
        || trim.starts_with("<HTML")
}

/// Seed request paths: `{canary}` is URL-encoded by the caller when needed.
pub fn seed_paths(canary: &str, max: usize) -> Vec<String> {
    let enc = urlencoding_minimal(canary);
    let mut paths = vec![
        format!("/?q={enc}"),
        format!("/search?q={enc}"),
        format!("/error?msg={enc}"),
        format!("/{enc}"),
        format!("/preview?url={enc}"),
        format!("/sandbox-preview/{enc}/x"),
    ];
    paths.truncate(max.max(1));
    paths
}

fn urlencoding_minimal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn header_value(headers: &str, name: &str) -> Option<String> {
    let want = name.to_ascii_lowercase();
    for line in headers.lines() {
        if let Some((n, v)) = line.split_once(':') {
            if n.trim().eq_ignore_ascii_case(&want) {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn split_http(raw: &str) -> (String, String, String) {
    let (head, body) = if let Some(i) = raw.find("\r\n\r\n") {
        (&raw[..i], &raw[i + 4..])
    } else if let Some(i) = raw.find("\n\n") {
        (&raw[..i], &raw[i + 2..])
    } else {
        (raw, "")
    };
    let status = head.lines().next().unwrap_or("").to_string();
    (status, head.to_string(), body.to_string())
}

async fn http_get(cfg: &Config, port: u16, path: &str) -> Option<(String, Option<String>, String)> {
    let addr = format!("{}:{}", cfg.scan.host, port);
    let http_to = Duration::from_millis(cfg.scan.http_timeout_ms);
    let result = timeout(http_to, async {
        let mut stream = TcpStream::connect(&addr).await.ok()?;
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nUser-Agent: mcp-guard/0.1\r\nAccept: text/html,*/*\r\n\r\n",
            cfg.scan.host
        );
        stream.write_all(req.as_bytes()).await.ok()?;
        let mut buf = vec![0u8; 16384];
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
        let text = String::from_utf8_lossy(&buf[..n]).into_owned();
        let (status, head, body) = split_http(&text);
        if !(status.starts_with("HTTP/1.") || status.starts_with("HTTP/2")) {
            return None;
        }
        let ct = header_value(&head, "content-type");
        Some((status, ct, body))
    })
    .await;
    match result {
        Ok(v) => v,
        Err(_) => None,
    }
}

/// Probe one open port for reflected XSS. Returns `None` when disabled or no HTML hit.
pub async fn probe_port(cfg: &Config, port: u16) -> Option<XssReflectProbe> {
    if !cfg.scan.xss_reflect {
        return None;
    }
    let token = format!("{:08x}", port as u32 ^ 0x6d6778);
    let canary = make_canary(&token);
    let paths = seed_paths(&canary, cfg.scan.xss_max_probes_per_port);

    let mut saw_html = false;
    let mut best_escaped: Option<XssReflectProbe> = None;

    for path in paths {
        let Some((_status, ct, body)) = http_get(cfg, port, &path).await else {
            continue;
        };
        if !looks_like_html(ct.as_deref(), &body) {
            continue;
        }
        saw_html = true;
        match classify_reflection(&body, &canary) {
            ReflectKind::Unescaped => {
                return Some(XssReflectProbe {
                    outcome: "unescaped",
                    path,
                    canary,
                });
            }
            ReflectKind::Escaped => {
                if best_escaped.is_none() {
                    best_escaped = Some(XssReflectProbe {
                        outcome: "escaped",
                        path,
                        canary: canary.clone(),
                    });
                }
            }
            ReflectKind::None => {}
        }
    }

    if let Some(p) = best_escaped {
        return Some(p);
    }
    if saw_html {
        return Some(XssReflectProbe {
            outcome: "html_no_reflect",
            path: "/".into(),
            canary,
        });
    }
    None
}

/// Risk flag when outcome is unescaped.
pub fn risk_flag_for(probe: Option<&XssReflectProbe>) -> Option<&'static str> {
    match probe.map(|p| p.outcome) {
        Some("unescaped") => Some("xss_reflected_unescaped"),
        _ => None,
    }
}
