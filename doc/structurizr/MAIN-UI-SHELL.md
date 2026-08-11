# Main dashboard shell (REQ-MAIN-UI)

Primary surface: **risk list** (app · MCP · human-readable flags).

## Startup

On dashboard open (`Init`), run **one** scan automatically (same path as「立即扫描」).

## Actions (home toolbar)

| Control | Meaning |
|---------|---------|
| 立即扫描 | Run one scan+watch tick; refresh risk cards |
| 扩展 | Secondary shell for local plugins |

## Window chrome

Frameless dashboard (`decorations=false`) with HTML titlebar + custom scrollbars.
Brand shield logo is the window / tray icon (`ui/brand/logo.png`).
Open log / pause toasts remain tray-menu only.

### Minimize / close → tray

When launched with the tray agent (`mcp-guard tray`):

- Titlebar **minimize** and **close (×)** **hide** the window to the system tray (agent keeps running).
- Tray menu **打开主界面** / left-click tray icon **shows** and focuses the existing window (no second instance).
- Real exit is tray menu **退出** only.

Standalone `dashboard` CLI (no tray) still closes the process on ×.

## Plugins shell

`view-plugins` is an abstract hub. Today: **密钥保险箱** (vault). Future plugins register as cards here — vault is not a top-level peer of Scan.
