# REQ-VAULT-MCP-UI — mapping

| Logical | Preview / DOM | Native IPC / bridge |
|---------|---------------|---------------------|
| Strip | `#vault-agent` | `data-state` = probe state |
| Title | `#vault-agent-title` | i18n `vault.agent_title` |
| Status line | `#vault-mcp-status` | `mcpGuardVaultMcpApply` |
| Detail | `#vault-mcp-detail` | status.detail + mcp_json_path |
| CTA | `#vault-mcp-cta` | `{"action":"vault-mcp-install"}` |
| Probe on open | vault view show | `{"action":"vault-mcp-status"}` |
| Push status | JS bridge | `window.mcpGuardVaultMcpApply(status)` |

Status payload (`CursorMcpStatus`):

```json
{
  "state": "no_cursor|not_installed|installed|broken",
  "detail": "…",
  "mcp_json_path": "~/.cursor/mcp.json",
  "cursor_detected": true,
  "exe": "/path/to/mcp-guard"
}
```

Server key written: `mcp-guard-vault` → `command` + `args: ["vault-mcp"]`.
