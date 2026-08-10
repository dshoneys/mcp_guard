# REQ-TRAY-UI — information architecture

## Tray menu (top → bottom)

1. **Status header** (non-action): severity dot + label from state table
2. **Open audit log** — secondary line = audit file path basename
3. **Scan now**
4. **Mute alerts (1h)**
5. **Quit**

## Data needed (contracts)

| Logical field | Source |
|---------------|--------|
| `severity` | Derived from recent `exposure_alert` / `activity_alert` + mute |
| `audit_path` | Config |
| `last_scan_at` | Latest audit `scan` / report timestamp |
| `mute_until` | ui_shell local state (or small sidecar; not plugin→plugin) |

## Out of menu (v1)

- Port list detail
- Allowlist editor
- Theme picker (tokens via `ui/` only)
