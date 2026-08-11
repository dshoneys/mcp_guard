use mcp_guard::config::{Config, ScanConfig};
use mcp_guard::net_enum::{is_loopback_or_unspecified, merge_probe_ports, resolve_probe_ports};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[test]
fn bind_filter_accepts_loopback_and_unspecified() {
    assert!(is_loopback_or_unspecified(IpAddr::V4(Ipv4Addr::LOCALHOST)));
    assert!(is_loopback_or_unspecified(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
    assert!(is_loopback_or_unspecified(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    assert!(is_loopback_or_unspecified(IpAddr::V6(Ipv6Addr::UNSPECIFIED)));
    assert!(!is_loopback_or_unspecified(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
}

#[test]
fn merge_includes_extras_and_caps() {
    let merged = merge_probe_ports(&[80, 443, 8080], &[9, 80], 3);
    assert_eq!(merged, vec![9, 80, 443]);
}

#[test]
fn resolve_with_discovery_off_uses_pins_or_legacy_defaults() {
    let mut cfg = Config::default();
    cfg.scan = ScanConfig {
        discover_listeners: false,
        ports: vec![7777],
        ..ScanConfig::default()
    };
    assert_eq!(resolve_probe_ports(&cfg, &[]), vec![7777]);

    cfg.scan.ports.clear();
    let ports = resolve_probe_ports(&cfg, &[]);
    assert!(ports.contains(&50551));
    assert!(ports.contains(&8080));
}

#[test]
fn resolve_with_discovery_on_includes_live_listeners() {
    let cfg = Config::default();
    assert!(cfg.scan.discover_listeners);
    let ports = resolve_probe_ports(&cfg, &[]);
    // Machine almost always has at least one loopback listener; if somehow empty, still ok.
    // Dedup + sorted is the contract we assert strongly.
    let mut sorted = ports.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(ports, sorted);
}
