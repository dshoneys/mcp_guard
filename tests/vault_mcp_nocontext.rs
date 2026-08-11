use mcp_guard::config::VaultConfig;
use mcp_guard::vault::mcp::{assert_nocontext, dispatch_tool_for_test};
use mcp_guard::vault::{scrub_secret, Vault};
use serde_json::json;
use std::fs;

fn vault(tag: &str) -> Vault {
    let dir = std::env::temp_dir().join(format!(
        "mcp-guard-vault-mcp-{}-{}-{}",
        tag,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::create_dir_all(&dir);
    let cfg = VaultConfig {
        store_path: dir.join("vault.enc"),
        key_path: dir.join("vault.key"),
        ref_ttl_secs: 60,
    };
    let v = Vault::open(&cfg).unwrap();
    v.put("api", "super-secret-value").unwrap();
    v
}

#[test]
fn issue_ref_payload_has_no_secret_fields() {
    let v = vault("issue");
    let res = dispatch_tool_for_test(&v, "vault_issue_ref", json!({"name":"api"})).unwrap();
    let text = res["content"][0]["text"].as_str().unwrap();
    assert!(!text.contains("super-secret-value"));
    assert!(!text.contains("\"value\""));
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_nocontext(&parsed).unwrap();
    assert!(parsed["ref"].as_str().unwrap().starts_with("vr_"));
}

#[test]
fn vault_get_forbidden() {
    let v = vault("forbid");
    let err = dispatch_tool_for_test(&v, "vault_get", json!({"name":"api"})).unwrap_err();
    assert!(err.to_string().contains("forbidden") || err.to_string().contains("NoContext"));
}

#[test]
fn scrubber_used_on_run_output_shape() {
    let scrubbed = scrub_secret("out=super-secret-value ok", "super-secret-value");
    assert!(!scrubbed.contains("super-secret-value"));
    assert!(scrubbed.contains("***REDACTED***"));
}

#[test]
fn list_payload_names_only() {
    let v = vault("list");
    let res = dispatch_tool_for_test(&v, "vault_list", json!({})).unwrap();
    let text = res["content"][0]["text"].as_str().unwrap();
    assert!(!text.contains("super-secret-value"));
    assert!(text.contains("api"));
}
