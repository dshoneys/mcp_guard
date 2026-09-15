# REQ-VAULT-MCP-UI — UX brief

## Goal

On the **密钥保险箱** view, the operator can see whether **local Cursor** is available and whether **MCP Guard `vault-mcp`** is wired into Cursor’s MCP config — and **one-click install / repair** that wiring without editing JSON by hand.

## Why

Secrets in the vault are useless to agents until `mcp-guard vault-mcp` is registered. Today that step is manual (`REQ-VAULT-MCP`). Operators who already use the dashboard should finish onboarding **in the same surface**.

## Users

Same as REQ-VAULT-UI: solo operator on a machine that may run Cursor.

## Jobs to be done

1. Open 保险箱 and immediately see **Cursor / vault-mcp status** (not buried in docs)
2. If Cursor is missing → clear explanation (no fake “Install Cursor” download flow in v1)
3. If Cursor present but MCP not wired (or broken) → **一键安装 / 修复**
4. If already wired → show confirmation; optional **重新写入** if path/exe drifted
5. Never expose vault secret plaintext as part of this flow

## Relationship to other reqs

| Req | Role |
|-----|------|
| REQ-VAULT-UI | Secrets CRUD (names only) |
| REQ-VAULT-MCP | stdio server + NoContext contract |
| **REQ-VAULT-MCP-UI** | Detect Cursor + install/repair `mcp.json` entry for `vault-mcp` |

## Target config (v1)

- **Primary:** user-global `~/.cursor/mcp.json` (`mcpServers` map)
- **Server key:** `mcp-guard-vault` (stable id; do not invent aliases per install)
- **Command:** absolute path to current `mcp-guard` executable + args `["vault-mcp"]`
- **Non-goals for path scope:** project-local `.cursor/mcp.json` (may be v2); other hosts (Claude Desktop, etc.)

## Non-goals (v1)

- Installing / downloading Cursor itself
- Editing arbitrary MCP servers unrelated to mcp-guard
- Cloud sync of mcp.json
- Revealing secrets while installing
- Auto-restart of Cursor (tell operator to reload MCP / restart Cursor)

## Success

Operator: 保险箱 → sees status → one click → Cursor can call `vault_*` tools with NoContext payloads; toast + on-page status confirm success/failure.
