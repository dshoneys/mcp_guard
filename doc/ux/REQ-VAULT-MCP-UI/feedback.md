# REQ-VAULT-MCP-UI — feedback

Silent install/probe forbidden.

| Event | On-page | OS toast |
|-------|---------|----------|
| Enter vault / after install | Status strip updates (`states.md`) | — |
| Install/repair success | Strip → `installed` | Success title + short body (path optional) |
| Install/repair failure | Strip → `error` + reason | Failure title + error string |
| No Cursor | Strip → `no_cursor` | — (no toast spam) |
| Probe failure (IO) | Strip → `error` | Only if operator clicked install/repair |

Never echo vault secret plaintext in strip, toast, or logs at info level.
