# REQ-TRAY-UI — mapping (preview ↔ config ↔ code)

## Tokens

| Logical | CSS (`ui/tokens.css`) | TOML (`ui/default.toml`) |
|---------|----------------------|---------------------------|
| background | `--mg-bg` | `theme.bg` |
| surface | `--mg-surface` | `theme.surface` |
| text | `--mg-text` | `theme.text` |
| muted | `--mg-muted` | `theme.muted` |
| ok | `--mg-ok` | `theme.ok` |
| warn | `--mg-warn` | `theme.warn` |
| danger | `--mg-danger` | `theme.danger` |
| accent | `--mg-accent` | `theme.accent` |

## Copy / states

| state_id | Preview tab | TOML key | `TrayMenuModel.header_label` |
|----------|-------------|----------|------------------------------|
| `idle` | Idle | `tray.copy.idle` | default / muted suffix |
| `exposure` | Exposure | `tray.copy.exposure` | warn |
| `activity` | Activity | `tray.copy.activity` | danger |

## Actions

| Preview `data-action` | `TrayActionId` | Menu label |
|----------------------|----------------|------------|
| `open-audit` | `OpenAudit` | Open audit log |
| `run-scan` | `ScanNow` | Scan now |
| `mute` | `Mute` | Mute alerts (1h) |
| `quit` | `Quit` | Quit |

## DOM hooks (preview)

| Selector / attr | Meaning |
|-----------------|--------|
| `[data-widget="tray-menu"]` | Menu container |
| `#status-dot` | Severity indicator |
| `#status-label` | Header copy |
| `li button[data-action]` | Action row |
