# REQ-TRAY-UI — UX acceptance

- [x] States `idle` / `exposure` / `activity` defined with severity ranking
- [x] Transient `scanning` feedback defined (tooltip)
- [x] Menu actions: Open audit, Scan now, Mute 1h, Quit
- [x] Mute does not stop auditing
- [x] **Feedback amendment**: Scan now always toasts; escalation toasts once; log-only ≠ UX ([`feedback.md`](./feedback.md))
- [x] Preview mock exists at `ui/preview/REQ-TRAY-UI/` for shared vocabulary
- [x] Non-goals explicit (no hard gate UI / no full panel yet)
- [x] Lead sets `ux_status: accepted` in `requirements.json` after review

## Lead decision

**Accepted** (2026-08-10) for menu model + StatusSource.  

**Amended** (2026-08-10 evening): feedback loops are part of UX contract — silent Scan now is a defect. Implementation must match [`feedback.md`](./feedback.md) (toasts already landed as hotfix; keep UX doc as authority).
