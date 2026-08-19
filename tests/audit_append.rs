use mcp_guard::audit;
use mcp_guard::config::AuditConfig;
use std::fs;
use std::path::PathBuf;

#[test]
fn append_creates_parent_and_valid_jsonl() {
    let dir = std::env::temp_dir().join(format!(
        "mcp-guard-audit-test-{}",
        std::process::id()
    ));
    let path = dir.join("nested").join("audit.jsonl");
    let _ = fs::remove_dir_all(&dir);

    let cfg = AuditConfig {
        path: path.clone(),
        ..AuditConfig::default()
    };
    audit::append(&cfg, "scan", serde_json::json!({"ok": true})).unwrap();
    audit::append(&cfg, "exposure_alert", serde_json::json!({"n": 1})).unwrap();

    let text = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 2);

    let v0: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v0["kind"], "scan");
    assert!(v0["ts"].as_str().unwrap().len() > 0);
    assert_eq!(v0["detail"]["ok"], true);

    let v1: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(v1["kind"], "exposure_alert");

    let _ = fs::remove_dir_all(&dir);
    let _: PathBuf = path;
}
