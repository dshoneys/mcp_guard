use mcp_guard::audit::snapshot_from_jsonl;
use mcp_guard::contracts::AlertSnapshot;
use std::fs;
use std::io::Write;

#[test]
fn missing_file_is_empty_snapshot() {
    let path = std::env::temp_dir().join(format!(
        "mcp-guard-missing-status-{}.jsonl",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);
    let snap = snapshot_from_jsonl(&path).unwrap();
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

    let snap = snapshot_from_jsonl(&path).unwrap();
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
    let snap = snapshot_from_jsonl(&path).unwrap();
    assert_eq!(snap.exposure_count, 2);
    assert_eq!(snap.activity_count, 0);
    let _ = fs::remove_file(&path);
}
