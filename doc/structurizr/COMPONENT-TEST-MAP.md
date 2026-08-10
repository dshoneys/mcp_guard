# Component ↔ requirement ↔ test map

Source of truth: `model/requirements.json`.  
\(R_{\mathrm{manual}}\) and UX/UI backlogs: `python scripts/adl_check.py`.

| req id | component | test_kind | ui / ux | tests / note | status |
|--------|-----------|-----------|---------|--------------|--------|
| REQ-SCAN-EXPOSURE | scan | unit | no | `tests/scan_http_headers.rs` | ✅ |
| REQ-SCAN-LIVE | scan | manual | no | WorkBuddy live | human |
| REQ-WATCH-ATTR | watch | unit | no | `tests/watch_allowlist.rs` | ✅ |
| REQ-WATCH-LIVE | watch | manual | no | PoC live | human |
| REQ-AUDIT-JSONL | audit | unit | no | `tests/audit_append.rs` | ✅ |
| REQ-SERVE-LOOP | runtime | contract | no | `tests/serve_once.rs`, `serve_cancel.rs` | ✅ |
| REQ-EXPOSURE-ALERT | runtime | unit | no | `tests/exposure_alert.rs` | ✅ |
| REQ-ACTIVITY-ALERT | runtime | unit | no | `tests/activity_alert.rs` | ✅ |
| REQ-GATE-HARD | gate | manual | no | OS block PoC | planned |
| REQ-CONFIG-TOML | config | unit | no | `tests/config_load.rs` | ✅ |
| REQ-TRAY-UI | ui_shell | manual | UX+UI accepted | native tray + preview | in progress (人测) |
| REQ-STATUS-JSONL | audit | unit | no | `tests/status_snapshot.rs` | ✅ |
| REQ-TRAY-MENU | ui_shell | unit | no | `tests/tray_menu.rs` | ✅ |
