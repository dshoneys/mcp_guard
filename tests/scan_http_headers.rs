use mcp_guard::contracts::McpProbe;
use mcp_guard::scan::{classify_risks, parse_http_response, HttpProbe};

#[test]
fn parse_acao_and_www_authenticate() {
    let raw = "HTTP/1.1 200 OK\r\n\
Server: ardot\r\n\
Access-Control-Allow-Origin: *\r\n\
\r\n\
{}";
    let h = parse_http_response(raw);
    assert!(h.status_line.contains("200"));
    assert_eq!(h.access_control_allow_origin.as_deref(), Some("*"));
    assert!(h.www_authenticate.is_none());
    assert_eq!(h.server.as_deref(), Some("ardot"));
}

#[test]
fn classify_cors_star_without_mcp_is_clean() {
    let http = HttpProbe {
        status_line: "HTTP/1.1 200 OK".into(),
        server: None,
        access_control_allow_origin: Some("*".into()),
        www_authenticate: None,
        body_snippet: String::new(),
    };
    let flags = classify_risks(50551, Some(&http), None);
    assert!(flags.is_empty());
}

#[test]
fn classify_bare_tcp_is_not_exposure() {
    let flags = classify_risks(8080, None, None);
    assert!(flags.is_empty());
}

#[test]
fn classify_plain_http_is_not_warning() {
    let http = HttpProbe {
        status_line: "HTTP/1.1 200 OK".into(),
        server: Some("nginx".into()),
        access_control_allow_origin: None,
        www_authenticate: None,
        body_snippet: "<html>hi</html>".into(),
    };
    let flags = classify_risks(18080, Some(&http), None);
    assert!(flags.is_empty());
}

#[test]
fn authenticate_on_http_without_mcp_is_clean() {
    let http = HttpProbe {
        status_line: "HTTP/1.1 401 Unauthorized".into(),
        server: None,
        access_control_allow_origin: Some("https://example.com".into()),
        www_authenticate: Some("Bearer".into()),
        body_snippet: String::new(),
    };
    let flags = classify_risks(3000, Some(&http), None);
    assert!(flags.is_empty());
}

#[test]
fn mcp_with_www_authenticate_skips_no_auth_hint() {
    let http = HttpProbe {
        status_line: "HTTP/1.1 200 OK".into(),
        server: None,
        access_control_allow_origin: Some("*".into()),
        www_authenticate: Some("Bearer".into()),
        body_snippet: String::new(),
    };
    let mcp = McpProbe {
        endpoint: "/mcp".into(),
        tool_count: 2,
        sample_tools: vec!["a".into(), "b".into()],
    };
    let flags = classify_risks(3000, Some(&http), Some(&mcp));
    assert!(flags.contains(&"mcp_tools_exposed"));
    assert!(flags.contains(&"cors_star"));
    assert!(!flags.contains(&"no_www_authenticate_hint"));
}
