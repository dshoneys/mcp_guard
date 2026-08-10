# Development framework status

## Health check (2026-08-10)

| 检查 | 结果 |
|------|------|
| `adl_check.py` | ✅ UX + UI accepted |
| `cargo test` | ✅（含 serve cancel） |
| Tray + agent | ✅ `tray` / `serve --tray` |

## Shipped

| 项 | 状态 |
|----|------|
| UX + UI REQ-TRAY-UI | ✅ accepted |
| Native tray | ✅ Win/macOS |
| Background agent under tray | ✅ Quit → cancel |
| `serve::tick_once` / `run_with_cancel` | ✅ |

## Open / manual

| 项 | 说明 |
|----|------|
| REQ-TRAY-UI 人测 | 托盘菜单 + 后台告警 |
| Live scan/watch | WorkBuddy 环境 |
| Hard gate | 未做 |
| 开机自启 / 安装包 | 未做 |

## Next

1. 人测托盘后 REQ-TRAY-UI → done  
2. Windows 安装/自启脚本  
3. Hard gate
