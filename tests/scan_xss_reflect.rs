use mcp_guard::xss_reflect::{
    classify_reflection, html_escape, looks_like_html, make_canary, risk_flag_for, seed_paths,
    ReflectKind,
};
use mcp_guard::contracts::XssReflectProbe;

#[test]
fn unescaped_raw_canary_is_xss() {
    let c = make_canary("deadbeef");
    assert!(c.contains("<>\"'"));
    let body = format!("<!DOCTYPE html><p>err: {c}</p>");
    assert_eq!(classify_reflection(&body, &c), ReflectKind::Unescaped);
}

#[test]
fn escaped_error_page_is_not_risk() {
    let c = make_canary("cafebabe");
    let body = format!(
        "<!DOCTYPE html><p>{}</p>",
        html_escape(&c)
    );
    assert_eq!(classify_reflection(&body, &c), ReflectKind::Escaped);
    let probe = XssReflectProbe {
        outcome: "escaped",
        path: "/error?msg=x".into(),
        canary: c,
    };
    assert!(risk_flag_for(Some(&probe)).is_none());
}

#[test]
fn spa_without_echo_is_none() {
    let c = make_canary("spa00001");
    let body = "<!DOCTYPE html><html><body><div id=app></div></body></html>";
    assert_eq!(classify_reflection(body, &c), ReflectKind::None);
}

#[test]
fn json_content_type_not_html() {
    assert!(!looks_like_html(
        Some("application/json"),
        "{\"q\":\"mgx<>\\\"'\"}"
    ));
    assert!(looks_like_html(
        Some("text/html; charset=utf-8"),
        "<html></html>"
    ));
    assert!(looks_like_html(None, "<!DOCTYPE html><html>"));
}

#[test]
fn unescaped_outcome_maps_to_flag() {
    let probe = XssReflectProbe {
        outcome: "unescaped",
        path: "/?q=x".into(),
        canary: make_canary("aa"),
    };
    assert_eq!(
        risk_flag_for(Some(&probe)),
        Some("xss_reflected_unescaped")
    );
}

#[test]
fn seed_paths_respect_cap() {
    let c = make_canary("t");
    assert_eq!(seed_paths(&c, 2).len(), 2);
    assert!(seed_paths(&c, 6)[0].contains('?'));
}

#[tokio::test]
async fn live_listener_unescaped_is_flagged() {
    use mcp_guard::config::Config;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let mut buf = [0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            let path = req
                .lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or("/");
            // Echo path (decoded-ish: leave %XX as-is still contains canary token after decode in probe URL)
            // Our probe URL-encodes markers; server echoes the raw request-target path string.
            // Canary is in query after decode by browser; we echo the request line path which has %3C etc.
            // Reflect by decoding common encodings for the test, or echo query decoded.
            let echoed = {
                let q = path.split('?').nth(1).unwrap_or("");
                let mut out = String::new();
                // crude: find q= value
                for part in q.split('&') {
                    if let Some(v) = part.strip_prefix("q=") {
                        out = percent_decode(v);
                        break;
                    }
                    if let Some(v) = part.strip_prefix("msg=") {
                        out = percent_decode(v);
                        break;
                    }
                }
                if out.is_empty() {
                    // path segment
                    let seg = path.trim_start_matches('/').split('/').next().unwrap_or("");
                    out = percent_decode(seg.split('?').next().unwrap_or(seg));
                }
                out
            };
            let body = format!("<!DOCTYPE html><html><body>{echoed}</body></html>");
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        }
    });

    let mut cfg = Config::default();
    cfg.scan.host = "127.0.0.1".into();
    cfg.scan.xss_reflect = true;
    cfg.scan.http_timeout_ms = 2000;

    let probe = mcp_guard::xss_reflect::probe_port(&cfg, port)
        .await
        .expect("xss probe");
    assert_eq!(probe.outcome, "unescaped");
    assert_eq!(
        risk_flag_for(Some(&probe)),
        Some("xss_reflected_unescaped")
    );
}

#[tokio::test]
async fn live_listener_escaped_not_flagged() {
    use mcp_guard::config::Config;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let mut buf = [0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            let path = req
                .lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or("/");
            let echoed = {
                let q = path.split('?').nth(1).unwrap_or("");
                let mut out = String::new();
                for part in q.split('&') {
                    if let Some(v) = part.strip_prefix("q=") {
                        out = percent_decode(v);
                        break;
                    }
                    if let Some(v) = part.strip_prefix("msg=") {
                        out = percent_decode(v);
                        break;
                    }
                }
                out
            };
            let safe = mcp_guard::xss_reflect::html_escape(&echoed);
            let body = format!("<!DOCTYPE html><p>{safe}</p>");
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = sock.write_all(resp.as_bytes()).await;
        }
    });

    let mut cfg = Config::default();
    cfg.scan.host = "127.0.0.1".into();
    cfg.scan.xss_reflect = true;
    cfg.scan.http_timeout_ms = 2000;

    let probe = mcp_guard::xss_reflect::probe_port(&cfg, port)
        .await
        .expect("xss probe");
    assert_eq!(probe.outcome, "escaped");
    assert!(risk_flag_for(Some(&probe)).is_none());
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(h) = h {
                if let Ok(v) = u8::from_str_radix(h, 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
