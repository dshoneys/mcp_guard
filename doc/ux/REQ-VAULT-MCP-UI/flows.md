# REQ-VAULT-MCP-UI — flows

```mermaid
flowchart TD
  open[Open vault view] --> probe[Probe local Cursor + mcp.json]
  probe --> status{Status}
  status -->|no_cursor| miss[Show: Cursor not detected]
  status -->|not_installed| ready[Show: install CTA]
  status -->|installed| ok[Show: connected + optional repair]
  status -->|broken| repair[Show: repair CTA + reason]
  ready --> clickInstall[Operator: 一键安装]
  repair --> clickRepair[Operator: 一键修复]
  ok --> clickRepair
  clickInstall --> write[Merge mcp-guard-vault into ~/.cursor/mcp.json]
  clickRepair --> write
  write --> toast[OS toast]
  toast --> refresh[Re-probe + update strip]
  miss --> docs[Hint: install Cursor, then return]
```

## Probe rules (logic)

1. **Cursor detected** if a known install signal exists (e.g. macOS `/Applications/Cursor.app`, or `~/.cursor` directory present). Prefer existence checks over process list.
2. **Installed** if `mcpServers["mcp-guard-vault"]` exists and `command` resolves to a runnable `mcp-guard` (or matches current exe) with `vault-mcp` in args.
3. **Broken** if key exists but command missing / not executable / args wrong.
4. Probe is **read-only**; install/repair is the only write path.
