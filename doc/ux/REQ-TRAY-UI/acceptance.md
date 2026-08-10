# REQ-TRAY-UI — UX acceptance

- [x] States `idle` / `exposure` / `activity` defined with severity ranking
- [x] Menu actions: Open audit, Scan now, Mute 1h, Quit
- [x] Mute does not stop auditing
- [x] Preview mock exists at `ui/preview/REQ-TRAY-UI/` for shared vocabulary
- [x] Non-goals explicit (no hard gate UI)
- [x] Lead sets `ux_status: accepted` in `requirements.json` after review

## Lead decision

**Accepted** (2026-08-10). Implement menu model + StatusSource + CLI `status` / tray loop stub. Native OS tray chrome remains \(R_{\mathrm{manual}}\).
