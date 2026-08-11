# REQ-MAIN-UI — feedback

## Principle

Same rule as tray: **no silent success** for operator actions. Main window adds an on-page **result strip** in addition to OS toast.

## Scan now

1. Immediate: status → `Scanning…`; Scan disabled; scan-result panel shows “扫描中…”
2. On finish: OS toast **and** scan-result panel filled (counts + headline) — this is the primary feedback surface
3. Re-enable button; refresh status strip from StatusSource

## Result strip (secondary actions)

Open audit / Mute / vault save-delete use a short confirm line under actions (not competing with scan panel).
