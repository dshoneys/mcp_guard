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
| Vault NoContext + `vault-mcp` | ✅ store/MCP/tests |
| UX + UI REQ-VAULT-UI | ✅ accepted（主面板） |

## Open / manual

| 项 | 说明 |
|----|------|
| REQ-TRAY-UI 人测 | 托盘 + **Scan now 必有 toast** |
| REQ-VAULT-UI / REQ-VAULT-MCP 人测 | 面板存钥 + Cursor 接 `vault-mcp`，确认上下文无明文 |
| Live scan/watch | WorkBuddy 环境 |
| Hard gate | 未做 |
| 告警详情窗 | 未做（下一 UX） |

## Lesson

首版 UX 只写了状态/菜单，**没写操作反馈** → 实现静默成功。已用 `doc/ux/REQ-TRAY-UI/feedback.md` 修正；后续 ui:true 需求验收必须含反馈通道。

## Next

1. 人测 toast / vault / vault-mcp 后关掉 manual reqs  
2. 可选：告警详情小窗（新 UX）  
3. Windows 安装/自启  
4. Hard gate
