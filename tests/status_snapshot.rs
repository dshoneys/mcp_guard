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
fn counts_alerts_and_last_scan() {
    let path = std::env::temp_dir().join(format!(
        "mcp-guard-status-{}.jsonl",
        std::process::id()
    ));
    let mut f = fs::File::create(&path).unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:00:00Z","kind":"scan","detail":{{}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:01:00Z","kind":"exposure_alert","detail":{{}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:02:00Z","kind":"activity_alert","detail":{{}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:03:00Z","kind":"scan","detail":{{}}}}"#
    )
    .unwrap();
    writeln!(
        f,
        r#"{{"ts":"2026-01-01T00:04:00Z","kind":"exposure_alert","detail":{{}}}}"#
    )
    .unwrap();

    let snap = snapshot_from_jsonl(&path).unwrap();
    assert_eq!(snap.exposure_count, 2);
    assert_eq!(snap.activity_count, 1);
    assert_eq!(snap.last_scan_at.as_deref(), Some("2026-01-01T00:03:00Z"));

    let _ = fs::remove_file(&path);
}
