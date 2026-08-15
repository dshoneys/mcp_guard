use mcp_guard::contracts::{AlertSnapshot, GuardSeverity, TrayActionId};
use mcp_guard::ui_shell::{
    build_menu, derive_state_id, is_muted, load_catalog, mute_until_one_hour_from, Catalog,
    DEFAULT_LOCALE,
};
use std::path::Path;
use std::time::{Duration, SystemTime};

#[test]
fn default_locale_is_zh_cn() {
    assert_eq!(DEFAULT_LOCALE, "zh-CN");
    let (id, cat) = load_catalog("zh-CN").expect("zh-CN catalog");
    assert_eq!(id, "zh-CN");
    assert!(cat.tray.scan_now.contains("扫描") || cat.tray.scan_now == "立即扫描");
    assert_eq!(cat.tray.quit, "退出");
}

#[test]
fn en_locale_loads() {
    let (id, cat) = load_catalog("en").expect("en catalog");
    assert_eq!(id, "en");
    assert_eq!(cat.tray.scan_now, "Scan now");
    assert_eq!(cat.tray.quit, "Quit");
}

#[test]
fn menu_order_zh() {
    let snap = AlertSnapshot {
        exposure_count: 1,
        activity_count: 0,
        last_scan_at: None,
    };
    let (_, cat) = load_catalog("zh-CN").unwrap();
    let model = build_menu(&snap, Path::new("logs/mcp-guard-audit.jsonl"), &cat, false);
    assert_eq!(model.state_id, "exposure");
    assert_eq!(model.severity, GuardSeverity::Warn);
    assert_eq!(model.header_label, cat.status.exposure);
    assert_eq!(model.items.len(), 5);
    assert_eq!(model.items[0].action, TrayActionId::OpenDashboard);
    assert_eq!(model.items[0].label, cat.tray.open_dashboard);
    assert_eq!(model.items[2].action, TrayActionId::ScanNow);
    assert_eq!(model.items[4].action, TrayActionId::Quit);
}

#[test]
fn derive_priority_activity_over_exposure() {
    let snap = AlertSnapshot {
        exposure_count: 3,
        activity_count: 1,
        last_scan_at: None,
    };
    assert_eq!(derive_state_id(&snap, false), "activity");
    assert_eq!(derive_state_id(&snap, true), "idle");
}

#[test]
fn mute_window() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    let until = mute_until_one_hour_from(now);
    assert!(is_muted(now + Duration::from_secs(10), Some(until)));
    assert!(!is_muted(now + Duration::from_secs(3601), Some(until)));
    assert!(!is_muted(now, None));
}

#[test]
fn builtin_default_is_chinese() {
    let cat = Catalog::default();
    assert!(cat.status.idle.contains("正常") || cat.status.idle.contains("MCP Guard"));
    assert_eq!(cat.tray.quit, "退出");
    assert!(cat.flags.cors_star.contains("CORS"));
}

#[test]
fn flag_pack_loaded_from_toml() {
    let (_, zh) = load_catalog("zh-CN").expect("zh-CN");
    assert!(zh.flags.known_workbuddy_ardot_port.contains("WorkBuddy"));
    assert!(zh.flags.mcp_jsonrpc_surface.contains("未保护"));
    assert!(zh.flags.mcp_tools_exposed.contains("进一步"));
    assert!(zh.flags.xss_reflected_unescaped.contains("反射"));
    let (_, en) = load_catalog("en").expect("en");
    assert!(en.flags.cors_star.contains("CORS"));
    assert!(en.flags.mcp_tools_exposed.contains("elevated"));
    assert!(en.flags.xss_reflected_unescaped.contains("XSS"));
    assert!(en.dashboard.risk_app_unknown.contains("Unknown"));
}
