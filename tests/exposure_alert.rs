use mcp_guard::contracts::{HttpProbe, PortFinding, ScanReport};
use mcp_guard::serve::{collect_exposures, should_raise_exposure_alert};

fn report_with_flags(flags: Vec<&'static str>) -> ScanReport {
    ScanReport {
        host: "127.0.0.1".into(),
        scanned_at: "t".into(),
        findings: vec![PortFinding {
            port: 50551,
            open: true,
            http: Some(HttpProbe {
                status_line: "HTTP/1.1 200 OK".into(),
                server: None,
                access_control_allow_origin: Some("*".into()),
                www_authenticate: None,
                body_snippet: String::new(),
            }),
            risk_flags: flags,
            mcp: None,
        }],
    }
}

#[test]
fn exposure_alert_when_risk_flags_present() {
    let report = report_with_flags(vec!["cors_star"]);
    assert!(should_raise_exposure_alert(&report, true));
    assert!(!should_raise_exposure_alert(&report, false));
    let exposures = collect_exposures(&report);
    assert_eq!(exposures.len(), 1);
    assert_eq!(exposures[0]["port"], 50551);
}

#[test]
fn no_exposure_alert_when_clean() {
    let report = report_with_flags(vec![]);
    assert!(!should_raise_exposure_alert(&report, true));
    assert!(collect_exposures(&report).is_empty());
}
