# xss-reflect-scan (pure JS)

Frontend twin of MCP Guard **REQ-SCAN-XSS-REFLECT** (`src/xss_reflect.rs`).

Detects **classic reflected XSS opportunity**: URL query/path canary echoed into `text/html` **without** HTML escaping.

## Install / deploy

Copy this folder, or import the ESM file from a static host:

```html
<script type="module">
  import { scanPorts, riskFlagFor } from "./index.js";
  const report = await scanPorts({ host: "127.0.0.1", ports: [3088, 61157] });
  console.log(report);
</script>
```

No build step. No npm required for browser use.

## API

| export | role |
|--------|------|
| `makeCanary` / `classifyReflection` / `htmlEscape` | Same rules as Rust |
| `probeOrigin({ baseUrl })` | One HTTP origin |
| `scanPorts({ host, ports })` | Many loopback ports |
| `riskFlagFor(probe)` | → `xss_reflected_unescaped` or `null` |

Outcomes: `unescaped` | `escaped` | `html_no_reflect` | `cors_blocked` | `none`.

Only **`unescaped`** is a scored risk (matches Rust default).

## Browser CORS limit

From a **public** page, `fetch('http://127.0.0.1:…')` usually cannot **read** the body unless the target sends `Access-Control-Allow-Origin` allowing you (or `*`). Then you get `cors_blocked` — not a false XSS hit.

Useful deployments:

1. Static page also served from loopback (or Electron/webview)
2. Demo against targets that already set `ACAO: *` (historical Ardot-style)
3. Node / Deno / Bun (no CORS) for CI / CLI wrappers

## Demo

Open [`demo.html`](./demo.html) via any static server (or `npx serve .`), enter ports, click Scan.

## Out of scope

Intentional preview HTML that runs scripts from disk (WorkBuddy StaticHtml) — not URL reflection.
