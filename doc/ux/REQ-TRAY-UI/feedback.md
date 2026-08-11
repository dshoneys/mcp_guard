# REQ-TRAY-UI — operator feedback (amendment)

> **Root cause note (2026-08-10):** First UX accept described *states* and *menu actions* but **not** the feedback loop for those actions. Implementers shipped silent Scan now → operators perceived “no reaction”. This is a **UX omission**, not only an engineering bug.

## Principle

Every operator-initiated action and every severity escalation MUST produce **at least one perceptible signal** within ~1s of completion (or immediately on start for long work). Silent success is forbidden for Scan now.

## Feedback channels (v1 — tray-only product)

| Channel | When to use | Notes |
|---------|-------------|--------|
| **Tray tooltip** | Immediate / transient | e.g. `Scanning…` while work runs |
| **Tray icon + menu header** | Persistent severity | idle / exposure / activity colors + copy |
| **OS notification (toast)** | Action result + escalation | Must fire even when result is “all clear” for Scan now |
| **Audit JSONL** | Durable record | Not a human feedback channel by itself |

Windows toast is the primary “I clicked something” acknowledgement. Tooltip alone is insufficient (easy to miss).

## Scan now — required interaction

```text
click Scan now
  → tooltip: "MCP Guard — Scanning…"   (immediate)
  → run scan + watch tick
  → toast ALWAYS:
       risk  → "Exposure alert" / "Suspicious activity" + short counts
       clear → "Scan complete" + "No new risk flags. Open services: N."
       error → "Scan failed" + error summary
  → refresh icon / menu header from StatusSource
```

**Acceptance:** Operator who never opens a terminal can tell that Scan now finished.

## Risk discovered (background agent)

```text
severity idle → exposure|activity  (and not muted)
  → toast once on escalation
  → icon + header update on next refresh (≤ refresh interval)
```

Do **not** toast on every periodic tick while already in warn/danger (avoid spam). Re-toast only on:
- new escalation from idle, or
- explicit Scan now (always toasts result)

## Mute

```text
click Mute
  → header shows muted affordance
  → optional short toast: "Alerts muted for 1h (auditing continues)"
  → no further escalation toasts until unmute/expiry
```

## Open audit

```text
click Open audit
  → OS reveals audit file (Explorer select)
  → if reveal fails → toast with error (not silent)
```

## What UX originally missed

1. Scan now success criteria with **no visible change** when idle  
2. Distinction between **durable state** (icon) and **event feedback** (toast)  
3. Explicit ban on “log-only” as the only feedback for human actions  

## Non-goals (still)

- Full alert detail window (future panel req)
- Notification action buttons that deep-link into a GUI (optional later)
