use mcp_guard::contracts::{HttpProbe, McpProbe};
use mcp_guard::scan::{classify_risks, parse_http_response, parse_mcp_tools_payload};

#[test]
fn plain_http_is_not_a_warning() {
    let http = HttpProbe {
        status_line: "HTTP/1.1 200 OK".into(),
        server: Some("nginx".into()),
        access_control_allow_origin: None,
        www_authenticate: None,
        body_snippet: "<html>hi</html>".into(),
    };
    let flags = classify_risks(18080, Some(&http), None);
    assert!(flags.is_empty(), "mcp_guard only warns on MCP surfaces");
}

#[test]
fn cors_star_alone_is_not_a_warning() {
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
fn unprotected_mcp_tools_is_elevated() {
    let http = HttpProbe {
        status_line: "HTTP/1.1 200 OK".into(),
        server: None,
        access_control_allow_origin: None,
        www_authenticate: None,
        body_snippet: String::new(),
    };
    let mcp = McpProbe {
        endpoint: "/api/v1/mcp".into(),
        tool_count: 3,
        sample_tools: vec!["upload_images".into(), "read_file".into()],
    };
    let flags = classify_risks(18488, Some(&http), Some(&mcp));
    assert!(flags.contains(&"mcp_tools_exposed"));
    assert!(flags.contains(&"no_www_authenticate_hint"));
    assert!(!flags.contains(&"open_http_no_cors"));
}

#[test]
fn unprotected_mcp_jsonrpc_is_warning() {
    let mcp = McpProbe {
        endpoint: "/mcp".into(),
        tool_count: 0,
        sample_tools: vec![],
    };
    let flags = classify_risks(9000, None, Some(&mcp));
    assert!(flags.contains(&"mcp_jsonrpc_surface"));
    assert!(flags.contains(&"no_www_authenticate_hint"));
}

#[test]
fn mcp_with_cors_star_coflags() {
    let http = HttpProbe {
        status_line: "HTTP/1.1 200 OK".into(),
        server: None,
        access_control_allow_origin: Some("*".into()),
        www_authenticate: None,
        body_snippet: String::new(),
    };
    let mcp = McpProbe {
        endpoint: "/api/v1/mcp".into(),
        tool_count: 1,
        sample_tools: vec!["x".into()],
    };
    let flags = classify_risks(50551, Some(&http), Some(&mcp));
    assert!(flags.contains(&"mcp_tools_exposed"));
    assert!(flags.contains(&"cors_star"));
    assert!(flags.contains(&"known_workbuddy_ardot_port"));
}

#[test]
fn parse_tools_list_json() {
    let body = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"a"},{"name":"b"},{"name":"c"}]}}"#;
    let (n, names) = parse_mcp_tools_payload(body).expect("tools");
    assert_eq!(n, 3);
    assert_eq!(names, vec!["a", "b", "c"]);
}

#[test]
fn parse_tools_list_sse() {
    let body = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[{\"name\":\"t1\"}]}}\n\n";
    let (n, names) = parse_mcp_tools_payload(body).expect("sse tools");
    assert_eq!(n, 1);
    assert_eq!(names, vec!["t1"]);
}

#[test]
fn non_http_garbage_not_scored() {
    let http = HttpProbe {
        status_line: "J\0\0\0mysql".into(),
        server: None,
        access_control_allow_origin: None,
        www_authenticate: None,
        body_snippet: String::new(),
    };
    let flags = classify_risks(3306, Some(&http), None);
    assert!(flags.is_empty());
}

#[test]
fn parse_http_still_works() {
    let raw = "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}";
    let h = parse_http_response(raw);
    assert!(h.status_line.contains("200"));
    assert_eq!(h.access_control_allow_origin.as_deref(), Some("*"));
}
