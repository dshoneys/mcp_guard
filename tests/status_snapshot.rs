use chrono::Utc;
use mcp_guard::audit::{latest_risks_from_jsonl, snapshot_from_jsonl, DEFAULT_ACTIVITY_ALERT_TTL_SECS};
use mcp_guard::contracts::{AlertSnapshot, RiskKind};
use std::fs;
use std::io::Write;

#[test]
fn missing_file_is_empty_snapshot() {
    let path = std::env::temp_dir().join(format!(
        "mcp-guard-missing-status-{}.jsonl",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);
    let snap = snapshot_from_jsonl(&path, DEFAULT_ACTIVITY_ALERT_TTL_SECS).unwrap();
    assert_eq!(snap, AlertSnapshot::default());
}

#[test]
fn current_posture_from_latest_scan_watch() {
    let path = std::env::temp_dir().join(format!(
        "mcp-guard-status-{}.jsonl",
        std::process::id()
    ));
    let mut f = fs::File::create(&path).unwrap();
    // Historical alerts must not stick forever.
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:00:00Z","kind":"scan","detail":{{"exposure_count":1,"open_services":[{{"port":50551}}]}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:01:00Z","kind":"exposure_alert","detail":{{}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:02:00Z","kind":"watch","detail":{{"alert_count":1}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:03:00Z","kind":"activity_alert","detail":{{}}}}"#
    )
    .unwrap();
    // Later clean tick clears chrome.
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:04:00Z","kind":"scan","detail":{{"exposure_count":0,"open_services":[]}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:05:00Z","kind":"watch","detail":{{"alert_count":0}}}}"#
    )
    .unwrap();

    let snap = snapshot_from_jsonl(&path, DEFAULT_ACTIVITY_ALERT_TTL_SECS).unwrap();
    assert_eq!(snap.exposure_count, 0);
    assert_eq!(snap.activity_count, 0);
    assert_eq!(snap.last_scan_at.as_deref(), Some("2026-01-01T00:04:00Z"));

    let _ = fs::remove_file(&path);
}

#[test]
fn latest_dirty_scan_keeps_exposure() {
    let path = std::env::temp_dir().join(format!(
        "mcp-guard-status-dirty-{}.jsonl",
        std::process::id()
    ));
    let mut f = fs::File::create(&path).unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:00:00Z","kind":"scan","detail":{{"exposure_count":2}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:01:00Z","kind":"watch","detail":{{"alert_count":0}}}}"#
    )
    .unwrap();
    let snap = snapshot_from_jsonl(&path, DEFAULT_ACTIVITY_ALERT_TTL_SECS).unwrap();
    assert_eq!(snap.exposure_count, 2);
    assert_eq!(snap.activity_count, 0);
    let _ = fs::remove_file(&path);
}

#[test]
fn recent_activity_alert_survives_clean_watch_for_dashboard() {
    let path = std::env::temp_dir().join(format!(
        "mcp-guard-recent-activity-{}.jsonl",
        std::process::id()
    ));
    let ts = Utc::now().to_rfc3339();
    let mut f = fs::File::create(&path).unwrap();
    writeln!(
        f,
        r#"{{"ts":"{ts}","kind":"scan","detail":{{"exposure_count":0,"open_services":[]}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"{ts}","kind":"watch","detail":{{"alert_count":1}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"{ts}","kind":"activity_alert","detail":{{"alert_count":1,"risks":[{{"app":"msedge.exe","flags":["unknown_client"],"kind":"activity","mcp":"本机 loopback MCP 表面 :3797","note":"连入 127.0.0.1:60447 → 127.0.0.1:3797（监听方：node.exe）","port":3797}}]}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"{ts}","kind":"watch","detail":{{"alert_count":0}}}}"#
    )
    .unwrap();

    let snap = snapshot_from_jsonl(&path, DEFAULT_ACTIVITY_ALERT_TTL_SECS).unwrap();
    assert_eq!(snap.activity_count, 1);

    let risks = latest_risks_from_jsonl(&path, DEFAULT_ACTIVITY_ALERT_TTL_SECS).unwrap();
    assert_eq!(risks.len(), 1);
    assert_eq!(risks[0].kind, RiskKind::Activity);
    assert_eq!(risks[0].port, 3797);
    assert_eq!(risks[0].app, "msedge.exe");
    assert!(risks[0].note.contains("连接已断开"));

    let _ = fs::remove_file(&path);
}
