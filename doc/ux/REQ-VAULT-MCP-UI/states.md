# REQ-VAULT-MCP-UI — states

| state_id | Strip meaning | Primary control |
|----------|---------------|-----------------|
| `probing` | 正在检测… | disabled |
| `no_cursor` | 未检测到本机 Cursor | none (hint only) |
| `not_installed` | 已检测到 Cursor，尚未接入 vault-mcp | 一键安装 |
| `installed` | 已接入 vault-mcp | 可选：重新写入 |
| `broken` | 配置存在但不可用（路径/参数错误） | 一键修复 |
| `busy` | 正在写入 mcp.json… | disable CTA |
| `error` | 上次写入失败（权限/JSON 损坏等） | retry CTA + reason |

`probing` / `busy` are transient overlays on the same strip; after completion return to a durable state above.
