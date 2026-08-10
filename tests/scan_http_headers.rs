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
fn classify_cors_star_and_no_auth_hint() {
    let http = HttpProbe {
        status_line: "HTTP/1.1 200 OK".into(),
        server: None,
        access_control_allow_origin: Some("*".into()),
        www_authenticate: None,
        body_snippet: String::new(),
    };
    let flags = classify_risks(50551, Some(&http));
    assert!(flags.contains(&"cors_star"));
    assert!(flags.contains(&"no_www_authenticate_hint"));
    assert!(flags.contains(&"known_workbuddy_ardot_port"));
}

#[test]
fn classify_tcp_open_without_http() {
    let flags = classify_risks(8080, None);
    assert_eq!(flags, vec!["tcp_open_non_http_or_timeout"]);
}

#[test]
fn authenticate_header_suppresses_no_auth_hint() {
    let http = HttpProbe {
        status_line: "HTTP/1.1 401 Unauthorized".into(),
        server: None,
        access_control_allow_origin: Some("https://example.com".into()),
        www_authenticate: Some("Bearer".into()),
        body_snippet: String::new(),
    };
    let flags = classify_risks(3000, Some(&http));
    assert!(!flags.contains(&"cors_star"));
    assert!(!flags.contains(&"no_www_authenticate_hint"));
}
