use mcp_guard::contracts::{ConnectionPeer, PeerProcess, PortWatch, WatchReport};
use mcp_guard::serve::should_raise_activity_alert;

#[test]
fn activity_alert_when_alert_count_positive() {
    let report = WatchReport {
        watched_at: "t".into(),
        alert_count: 2,
        ports: vec![PortWatch {
            port: 50551,
            listeners: vec![],
            peers: vec![ConnectionPeer {
                local: "127.0.0.1:1".into(),
                remote: "127.0.0.1:2".into(),
                state: "ESTABLISHED".into(),
                processes: vec![PeerProcess {
                    pid: 1,
                    name: "chrome".into(),
                    exe: None,
                    allowed: false,
                }],
                unknown_client: true,
            }],
        }],
    };
    assert!(should_raise_activity_alert(&report));
}

#[test]
fn no_activity_alert_when_zero() {
    let report = WatchReport {
        watched_at: "t".into(),
        alert_count: 0,
        ports: vec![],
    };
    assert!(!should_raise_activity_alert(&report));
}
