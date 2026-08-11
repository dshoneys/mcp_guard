# REQ-MAIN-UI — UX brief

## Goal

A **primary window** (not only tray) where the operator can understand MCP Guard health, see recent risk signal, and run the same actions as the tray — with clear feedback.

## Why

Tray-only product failed the “glanceable + actionable” job: operators cannot discover status without hunting the notification area, and silent menu actions felt broken. The tray remains the **resident affordance**; the main window is the **workspace**.

## Users

Same as REQ-TRAY-UI: solo operator / security reviewer on a machine running local MCP servers.

## Jobs to be done

1. Open a durable main surface from tray (or `mcp-guard dashboard`)
2. See current severity + last scan time + counts at a glance
3. Run Scan now / Open audit / Mute and **perceive results** (see feedback.md)
4. Leave the window; agent keeps running in tray

## Relationship to tray

| Surface | Role |
|---------|------|
| Tray | Always-on; severity icon; quick actions; OS toasts |
| Main window | Primary reading + working surface; same actions; richer status |

Closing the window **must not** quit the agent (Quit stays tray-only or explicit in window footer).

## Non-goals (v1)

- Multi-page settings / allowlist editor
- Full JSONL browser / log search
- Hard-gate controls (REQ-GATE-HARD)
- Account / cloud sync

## Success

Operator can work without a terminal: open dashboard → understand state → scan → see toast + on-page result strip.
