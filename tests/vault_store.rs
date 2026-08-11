use mcp_guard::config::VaultConfig;
use mcp_guard::vault::Vault;
use std::fs;

fn tmp_cfg(tag: &str) -> VaultConfig {
    let dir = std::env::temp_dir().join(format!("mcp-guard-vault-{tag}-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    VaultConfig {
        store_path: dir.join("vault.enc"),
        key_path: dir.join("vault.key"),
        ref_ttl_secs: 60,
    }
}

#[test]
fn put_list_resolve_roundtrip_without_plaintext_in_list() {
    let cfg = tmp_cfg("store");
    let v = Vault::open(&cfg).unwrap();
    v.put("openai", "sk-test-SECRET-999").unwrap();
    let list = v.list().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "openai");
    let raw = fs::read_to_string(&cfg.store_path).unwrap();
    assert!(!raw.contains("sk-test-SECRET-999"));
    assert_eq!(v.resolve_local("openai").unwrap(), "sk-test-SECRET-999");
    assert!(v.delete("openai").unwrap());
    assert!(v.list().unwrap().is_empty());
}

#[test]
fn issue_ref_has_no_value_and_resolves_locally() {
    let cfg = tmp_cfg("ref");
    let v = Vault::open(&cfg).unwrap();
    v.put("db", "hunter2").unwrap();
    let r = v.issue_ref("db").unwrap();
    assert!(r.ref_id.starts_with("vr_"));
    let info = v.ref_info(&r.ref_id).unwrap();
    assert_eq!(info["valid"], true);
    assert!(info.get("value").is_none());
    let (name, secret) = v.resolve_ref_local(&r.ref_id).unwrap();
    assert_eq!(name, "db");
    assert_eq!(secret, "hunter2");
}
