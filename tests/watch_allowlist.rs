use mcp_guard::config::GateConfig;
use mcp_guard::watch::is_allowed;

fn gate(names: &[&str]) -> GateConfig {
    GateConfig {
        allow_process_names: names.iter().map(|s| (*s).to_string()).collect(),
        alert_on_unknown: true,
    }
}

#[test]
fn allowlist_matches_process_name_substring() {
    let g = gate(&["WorkBuddy", "mcp-guard"]);
    assert!(is_allowed("WorkBuddy.exe", None, &g));
    assert!(is_allowed("mcp-guard", Some(r"C:\tools\mcp-guard.exe"), &g));
}

#[test]
fn allowlist_matches_exe_path() {
    let g = gate(&["codebuddy"]);
    assert!(is_allowed(
        "node",
        Some(r"C:\Users\x\AppData\Local\CodeBuddy\node.exe"),
        &g
    ));
}

#[test]
fn unknown_process_rejected() {
    let g = gate(&["WorkBuddy"]);
    assert!(!is_allowed("chrome.exe", Some(r"C:\Program Files\Google\Chrome\chrome.exe"), &g));
}
