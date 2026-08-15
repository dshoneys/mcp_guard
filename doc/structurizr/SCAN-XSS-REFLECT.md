# Scan: local reflected XSS opportunity (REQ-SCAN-XSS-REFLECT)

## Problem

mcp_guard already enumerates loopback listeners and scores **unprotected MCP**.
That pipeline misses a different class: **HTTP pages that reflect URL-controlled
input into `text/html` without escaping** (classic reflected XSS).

WorkBuddy StaticHtml / intentional preview script execution is **out of scope** —
those are stored/local-file content, not request reflection.

## Decision

After TCP open + optional MCP probe, run a **bounded reflected-XSS probe** on the
same port set:

1. GET a small set of seed URLs that place a unique **canary** in query and/or path.
2. Only continue when the response looks like HTML (`Content-Type: text/html` or
   body starts with `<!DOCTYPE` / `<html`).
3. If the **raw** canary (including marker characters `<>"'`) appears in the body →
   flag `xss_reflected_unescaped` (scored risk, independent of MCP).
4. If only HTML-escaped forms appear → record in `xss` DTO as `escaped`, **no** risk flag.
5. If no canary → `xss` absent or `none`; no risk flag.

Default: **only confirmed unescaped reflection raises `risk_flags`**. HTML surfaces
without reflection do not spam the dashboard.

## Config

```toml
[scan]
xss_reflect = true          # default true
xss_max_probes_per_port = 6 # safety cap
```

## Pure JS sibling

Same algorithm ships as browser/Node-deployable ESM under `web/xss-reflect-scan/`
for frontend demos. Browser reads of cross-origin loopback bodies require CORS
(or same-origin); the JS API reports `cors_blocked` when the body cannot be read.

## Paths

- Logic: `src/xss_reflect.rs` (used by `scan`)
- Wire: `src/scan.rs`
- DTO: `contracts::XssReflectProbe` on `PortFinding`
- JS: `web/xss-reflect-scan/`
- Tests: `tests/scan_xss_reflect.rs`
