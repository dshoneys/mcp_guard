# MCP Guard

<p align="center">
  <img src="ui/brand/logo.png" alt="MCP Guard logo" width="160" />
</p>

Local agent that **scans**, **watches**, **audits**, and (later) **gates** MCP / agent tool-call surfaces on the machine.

Repo: https://github.com/shinjiyu/mcp_guard

## How it works

MCP Guard does **not** sit in the browser. It runs as a host agent and looks at the shared bottleneck: **who can talk to local MCP ports**.

```text
  Malicious page / script / Electron / curl
              │
              ▼
        127.0.0.1:50551  (example: WorkBuddy Ardot MCP)
              │
    ┌─────────┴──────────────────────────────────┐
    │              MCP Guard (host)                │
    │  1. scan  — is the port open? CORS *? auth?│
    │  2. watch — which PID listens / connects?  │
    │  3. audit — JSONL trail                     │
    │  4. gate  — (later) drop unknown clients    │
    └─────────┬──────────────────────────────────┘
              ▼
         MCP server process
```

### ADL (architecture authority)

Multi-agent / O(1) rules live under [`doc/structurizr/`](doc/structurizr/README.md):

- Pursue **plugin out-degree O(1)** (`model/graph.json`)
- Compute **\(R_{\mathrm{manual}}=R\setminus R_U\)** (`model/requirements.json`) → human QA
- Gate: `python scripts/adl_check.py`
- Roles: [`AGENTS.md`](AGENTS.md) — **lead** 可直推 `master`；其它角色认领 Issue 后分支 + PR

### Pipeline

1. **Discover / probe (`scan`)**  
   Enumerate loopback listeners, HTTP fingerprint, then MCP `tools/list` probe.  
   Risk flags only for **unprotected MCP**: `mcp_jsonrpc_surface`, `mcp_tools_exposed`  
   (optional co-flags: `cors_star`, `no_www_authenticate_hint`, WorkBuddy pin).  
   If `scan.alert_on_exposure` (default true): also **EXPOSURE ALERT** + audit `exposure_alert`.

2. **Attribute (`watch`)**  
   Enumerate OS TCP tables (`netstat2`), map sockets → PID → process name/path (`sysinfo`).  
   Compare against `gate.allow_process_names`. Unknown clients → **ACTIVITY ALERT** + audit `activity_alert`.

3. **Record (`serve`)**  
   Loop: scan + watch → append `mcp-guard-audit.jsonl`.

4. **Hard gate (not yet)**  
   Windows WFP / macOS pf / Linux nftables (or a userspace proxy). Only after attribution is reliable.

| Alert | When |
|-------|------|
| `exposure_alert` | Surface looks exploitable (洞) |
| `activity_alert` | Non-allowlisted process is talking to the port (疑似利用中) |

Browser extensions are optional demos only — they cannot cover every client that can open loopback sockets.

## MVP status

| Capability | Status |
|------------|--------|
| `mcp-guard scan` | ✅ |
| `mcp-guard watch` | ✅ soft attribution |
| `mcp-guard serve` | ✅ scan + watch + audit |
| `mcp-guard serve --tray` | ✅ agent + native tray (Quit stops both) |
| `mcp-guard status` | ✅ menu model + audit snapshot JSON |
| `mcp-guard tray` | ✅ tray + main window + agent（默认中文；`--locale en`） |
| `mcp-guard dashboard` | ✅ 仅主窗口（无托盘；日常用 `tray`） |
| `mcp-guard vault` / `vault-mcp` | ✅ NoContext secret vault (refs + scrubbed runs) |
| Hard port/process block | ⏳ |
| Path / tool policy | ⏳ |
| Native tray UI pack | ✅ UX+UI accepted；人测见 \(R_{\mathrm{manual}}\) |

### Secret vault (NoContext)

Agents must **not** receive plaintext secrets as MCP tool results (that would enter chat context). See [`doc/structurizr/VAULT-NOCONTEXT.md`](doc/structurizr/VAULT-NOCONTEXT.md).

```bash
mcp-guard vault put openai --value "sk-..."
mcp-guard vault list
mcp-guard vault issue-ref openai
# Cursor / Claude Desktop mcp.json:
# { "command": "mcp-guard", "args": ["vault-mcp"] }
```

Tools: `vault_list`, `vault_issue_ref`, `vault_ref_info`, `vault_run_with_secret` — **no** `vault_get`.


## Build

```bash
cargo build --release
./target/release/mcp-guard scan
./target/release/mcp-guard watch
./target/release/mcp-guard serve --once
```

Optional config: copy `mcp-guard.toml.example` → `mcp-guard.toml`.

```bash
RUST_LOG=debug mcp-guard scan --ports 50551,52412
```

## License

MIT
