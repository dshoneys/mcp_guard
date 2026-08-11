# Scan escalation: MCP-only risk (REQ-SCAN-MCP-TOOLS)

`mcp_guard` scores **unprotected MCP surfaces only**. Plain HTTP / CORS on non-MCP ports is **not** a warning.

## Pipeline

1. **Enumerate** loopback/unspecified TCP `LISTEN`.
2. **HTTP GET /** — fingerprint only (status, ACAO, WWW-Authenticate, body). No risk flags by themselves.
3. **MCP tools probe** — POST JSON-RPC `tools/list` to candidate paths  
   (`/api/v1/mcp`, `/mcp`, `/`, `/message`) with MCP Accept headers.  
   `401` / `403` → treated as **protected** (no MCP risk).
4. If probe succeeds without auth:
   - `result.tools` non-empty → `mcp_tools_exposed` (**further / higher risk**)
   - JSON-RPC/MCP shape without tools → `mcp_jsonrpc_surface` (**warning**: unprotected MCP endpoint)
5. Optional co-flags **only when MCP was confirmed**:
   - `ACAO=*` → `cors_star` (browser-callable MCP)
   - no `WWW-Authenticate` → `no_www_authenticate_hint`
   - port `50551` → `known_workbuddy_ardot_port`

Bare non-HTTP TCP and ordinary local HTTP remain unscored.

## Paths

- `src/scan.rs` — probe + classify
- `src/contracts.rs` — optional `McpProbe` on `PortFinding`
- i18n `[flags]` — human-readable copy
