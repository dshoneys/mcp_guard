use mcp_guard::contracts::{HttpProbe, PortFinding, ScanReport};
use mcp_guard::serve::risk_details_from_scan;

#[test]
fn risk_details_list_app_mcp_and_flags() {
    let report = ScanReport {
        host: "127.0.0.1".into(),
        scanned_at: "t".into(),
        findings: vec![PortFinding {
            port: 50551,
            open: true,
            http: Some(HttpProbe {
                status_line: "HTTP/1.1 200".into(),
                server: None,
                access_control_allow_origin: Some("*".into()),
                www_authenticate: None,
                body_snippet: String::new(),
            }),
            risk_flags: vec!["cors_star", "no_www_authenticate_hint"],
            mcp: None,
        }],
    };
    let risks = risk_details_from_scan(&report);
    assert_eq!(risks.len(), 1);
    assert_eq!(risks[0].port, 50551);
    assert_eq!(risks[0].app, "WorkBuddy");
    assert!(risks[0].mcp.contains("WorkBuddy"));
    assert!(risks[0].flags.contains(&"cors_star".into()));
    assert!(risks[0].note.contains("ACAO=*"));
}
