use mcp_guard::contracts::{AlertSnapshot, GuardSeverity, TrayActionId};
use mcp_guard::ui_shell::{
    build_menu, derive_state_id, is_muted, mute_until_one_hour_from, TrayCopy,
};
use std::path::Path;
use std::time::{Duration, SystemTime};

fn copy() -> TrayCopy {
    TrayCopy::default()
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
fn menu_order_and_labels() {
    let snap = AlertSnapshot {
        exposure_count: 1,
        activity_count: 0,
        last_scan_at: None,
    };
    let model = build_menu(&snap, Path::new("logs/mcp-guard-audit.jsonl"), &copy(), false);
    assert_eq!(model.state_id, "exposure");
    assert_eq!(model.severity, GuardSeverity::Warn);
    assert_eq!(model.header_label, "Exposure alert");
    assert_eq!(model.items.len(), 4);
    assert_eq!(model.items[0].action, TrayActionId::OpenAudit);
    assert_eq!(
        model.items[0].subtitle.as_deref(),
        Some("mcp-guard-audit.jsonl")
    );
    assert_eq!(model.items[1].action, TrayActionId::ScanNow);
    assert_eq!(model.items[2].action, TrayActionId::Mute);
    assert_eq!(model.items[3].action, TrayActionId::Quit);
}

#[test]
fn mute_window() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    let until = mute_until_one_hour_from(now);
    assert!(is_muted(now + Duration::from_secs(10), Some(until)));
    assert!(!is_muted(now + Duration::from_secs(3601), Some(until)));
    assert!(!is_muted(now, None));
}
