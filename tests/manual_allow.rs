use mcp_guard::config::{add_manual_allow, load_manual_allows, merge_manual_allows, normalize_allow_token, Config, GateConfig, MANUAL_ALLOWS_FILE};
use mcp_guard::watch::is_allowed;
use std::fs;
use std::path::PathBuf;

#[test]
fn normalize_strips_exe_and_path() {
    assert_eq!(normalize_allow_token("msedge.exe"), "msedge");
    assert_eq!(
        normalize_allow_token(r"C:\Program Files\Edge\msedge.exe"),
        "msedge"
    );
}

#[test]
fn add_manual_allow_persists_and_merges() {
    let path = PathBuf::from(MANUAL_ALLOWS_FILE);
    let _ = fs::remove_file(&path);
    let mut cfg = Config::default();
    let token = add_manual_allow(&mut cfg, "msedge.exe").unwrap();
    assert_eq!(token, "msedge");
    assert!(cfg.gate.allow_process_names.iter().any(|x| x == "msedge"));
    let disk = load_manual_allows().unwrap();
    assert!(disk.iter().any(|x| x == "msedge"));
    let _ = fs::remove_file(&path);
}

#[test]
fn allowed_activity_client_is_not_unknown() {
    let mut gate = GateConfig::default();
    gate.allow_process_names.push("msedge".into());
    assert!(is_allowed("msedge.exe", None, &gate));
}

#[test]
fn merge_manual_allows_is_idempotent() {
    let path = PathBuf::from(MANUAL_ALLOWS_FILE);
    let _ = fs::remove_file(&path);
    let mut cfg = Config::default();
    add_manual_allow(&mut cfg, "msedge").unwrap();
    merge_manual_allows(&mut cfg.gate).unwrap();
    let count = cfg
        .gate
        .allow_process_names
        .iter()
        .filter(|x| x.eq_ignore_ascii_case("msedge"))
        .count();
    assert_eq!(count, 1);
    let _ = fs::remove_file(&path);
}
