# REQ-TRAY-UI — UX brief

## Goal

Resident **system tray** (or equivalent host chrome) so the operator can see MCP Guard health at a glance and act without opening a terminal.

## Users

- Solo operator running MCP clients (e.g. WorkBuddy) on the same machine
- Security reviewer verifying exposure/activity alerts

## Jobs to be done

1. Know whether Guard is running and whether anything is wrong
2. Open the JSONL audit trail quickly
3. Trigger an on-demand scan
4. Temporarily mute alerts
5. Quit the resident agent cleanly

## Non-goals (this req)

- Hard packet drop / OS firewall UI (REQ-GATE-HARD)
- Full settings GUI (future)
- Pixel-perfect native tray chrome in HTML preview (HITL / \(R_{\mathrm{manual}}\))

## Primary surface

OS tray icon + context menu. States and actions must match [`ui/preview/REQ-TRAY-UI`](../../../ui/preview/REQ-TRAY-UI/index.html) logically (idle / exposure / activity).

## Success

After UX accepted + behavior implemented: menu reflects latest alert severity; Open audit / Scan now / Mute / Quit work; manual QA on a real OS tray.
