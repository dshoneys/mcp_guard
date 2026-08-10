# Development framework status

## Health check (2026-08-10)

| 检查 | 结果 |
|------|------|
| `adl_check.py` 结构门禁 | ✅ 绿（无 hollow WARN） |
| `cargo test` | ✅ |
| contracts 调度 | ✅ `serve::run_with` + cli 接线 |
| 文档与 UX/UI 字段一致 | ✅ |

## Ready

| 块 | 状态 |
|----|------|
| ADL + O(1) / NoCross / \(R_{\mathrm{manual}}\) | ✅ |
| UX → 实现 → UI + Code-as-Design 预览 | ✅ |
| Agent 角色 + Lead Cursor | ✅ |
| Issue/PR/CI/CODEOWNERS | ✅ |
| \(R_U\) 真实单测 / 合约测 | ✅ |

## Open issues

| 严重度 | 问题 |
|--------|------|
| 中 | GitHub branch protection 需在仓库 Settings 手动开启 |
| 中 | `REQ-TRAY-UI`：有 HTML 预览样例，`doc/ux`/`doc/ui` 包仍 `needed` |
| 中 | `ui_shell` 出度已 = K(2)，再加依赖会破 O(1) |
| 低 | 嵌套在 kuroneko `experiments/` 下的独立 git；勿误提交进父仓 |
| 低 | HTML 预览不能严格还原原生托盘（HITL / \(R_{\mathrm{manual}}\)） |
| 低 | 尚无 import↔graph 自动漂移检查 |

## Next product work

1. 开 `[UX] REQ-TRAY-UI` 填 `doc/ux/REQ-TRAY-UI/`
2. 可选：import↔graph 漂移检查进 `adl_check.py`
