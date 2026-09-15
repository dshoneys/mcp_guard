# REQ-VAULT-MCP-UI — information architecture

## Placement

**Inside** `#view-vault` (REQ-VAULT-UI), **above** the secret form — onboarding before CRUD.

```text
[ Back ]
密钥保险箱
hint (NoContext)

┌─ Agent 接入 / Cursor ─────────────────────┐
│ status line + detail                       │
│ [ 一键安装 | 一键修复 | 重新写入 ]          │
└────────────────────────────────────────────┘

form: name + secret + Save
list: names only
```

## Copy principles

- One status sentence; one optional detail line (path / reason)
- CTA label matches state (`一键安装` / `一键修复` / `重新写入`)
- Never show secret values in this strip

## IPC (native)

| action | purpose |
|--------|---------|
| `vault-mcp-status` | probe → push status into UI |
| `vault-mcp-install` | merge/repair mcp.json → toast + refresh status |
