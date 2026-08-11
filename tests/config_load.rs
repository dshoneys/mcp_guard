use mcp_guard::config;
use std::fs;
use std::io::Write;

#[test]
fn missing_file_yields_defaults() {
    let cfg = config::load(None).unwrap();
    assert_eq!(cfg.scan.host, "127.0.0.1");
    assert!(cfg.scan.discover_listeners);
    assert!(cfg.scan.ports.is_empty());
    assert!(cfg.scan.alert_on_exposure);
}

#[test]
fn valid_toml_overrides_ports() {
    let dir = std::env::temp_dir().join(format!("mcp-guard-cfg-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("mcp-guard.toml");
    let mut f = fs::File::create(&path).unwrap();
    writeln!(
        f,
        r#"
[scan]
host = "127.0.0.1"
ports = [9]
alert_on_exposure = false
"#
    )
    .unwrap();

    let cfg = config::load(Some(&path)).unwrap();
    assert_eq!(cfg.scan.ports, vec![9]);
    assert!(!cfg.scan.alert_on_exposure);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn invalid_toml_errors() {
    let dir = std::env::temp_dir().join(format!("mcp-guard-cfg-bad-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("bad.toml");
    fs::write(&path, "[[[not valid").unwrap();
    assert!(config::load(Some(&path)).is_err());
    let _ = fs::remove_dir_all(&dir);
}
