# REQ-TRAY-UI — UI brief

## Design source

Code-as-Design preview: [`ui/preview/REQ-TRAY-UI/index.html`](../../../ui/preview/REQ-TRAY-UI/index.html)  
Tokens: [`ui/tokens.css`](../../../ui/tokens.css) ↔ [`ui/default.toml`](../../../ui/default.toml)

## Intent

Dark, compact tray-menu mock that communicates severity with a status dot + label. Native OS chrome will differ; **copy, action order, and state ids** are the contract.

## Visual rules

- Surface panel on dark stage; no card stack in the “hero” of the mock
- Severity: ok / warn / danger map to `--mg-ok` / `--mg-warn` / `--mg-danger`
- Four actions only (see UX IA): Open audit, Scan now, Mute 1h, Quit

## Native mapping

| Preview | Native |
|---------|--------|
| `data-widget="tray-menu"` | OS context menu on tray icon |
| `#status-label` | Tray tooltip + disabled header item (when supported) |
| `.dot` color | Icon tint / tooltip severity |
| `data-action=*` | `TrayActionId` menu ids |
