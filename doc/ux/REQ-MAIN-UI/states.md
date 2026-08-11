# REQ-MAIN-UI — states

| state_id | Window status strip | Accent |
|----------|---------------------|--------|
| `idle` | MCP Guard — OK | ok / green |
| `scanning` | Scanning… | accent / muted pulse |
| `exposure` | Exposure alert | warn |
| `activity` | Suspicious activity | danger |
| `muted` | (prior severity) + “muted” badge | muted |

`scanning` overlays the strip during Scan now; after completion revert to derived severity and fill **result strip**.
