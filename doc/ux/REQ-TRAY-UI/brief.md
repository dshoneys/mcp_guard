# REQ-TRAY-UI — UX brief

## Goal

Resident **system tray** (or equivalent host chrome) so the operator can see MCP Guard health at a glance and act without opening a terminal.

## Users

- Solo operator running MCP clients (e.g. WorkBuddy) on the same machine
- Security reviewer verifying exposure/activity alerts

## Jobs to be done

1. Know whether Guard is running and whether anything is wrong **without opening a terminal**
2. Open the JSONL audit trail quickly
3. Trigger an on-demand scan **and perceive that it finished** (clear or risk)
4. Temporarily mute alerts
5. Quit the resident agent cleanly

## Feedback (required)

Silent success is a UX defect. See [`feedback.md`](./feedback.md):

- Scan now always ends with an OS toast (clear / risk / error)
- Tooltip shows `Scanning…` while work runs
- Severity escalation (idle → risk) toasts once; icon/header hold persistent state

## Non-goals (this req)

- Hard packet drop / OS firewall UI (REQ-GATE-HARD)
- Full settings / alert-detail window (future panel req)
- Pixel-perfect native tray chrome in HTML preview (HITL / \(R_{\mathrm{manual}}\))

## Primary surface

OS tray icon + context menu + **OS notifications**. Logical states match [`ui/preview/REQ-TRAY-UI`](../../../ui/preview/REQ-TRAY-UI/index.html) (idle / exposure / activity).

## Success

Menu reflects severity; Scan now and escalations are perceptible; Open audit / Mute / Quit work; manual QA on a real OS tray.
