# MCP Guard

<p align="center">
  <img src="ui/brand/logo.png" alt="MCP Guard logo" width="160" />
</p>

<p align="center">
  <a href="README.md">中文</a> · <strong>English</strong>
</p>

Local agent that **scans**, **watches**, **audits**, and (later) **gates** MCP / agent tool-call surfaces on the machine.

Repo: https://github.com/shinjiyu/mcp_guard

Current version: `0.1.0-beta.1` (prerelease)

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
    │  1. scan  — enumerate; probe unprotected MCP │
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
- Roles: [`AGENTS.md`](AGENTS.md)

### Pipeline

1. **Discover / probe (`scan`)**  
   Enumerate loopback listeners, HTTP fingerprint, then MCP `tools/list` probe.  
   Risk flags **only for unprotected MCP**: `mcp_jsonrpc_surface`, `mcp_tools_exposed`  
   (optional co-flags when MCP confirmed: `cors_star`, `no_www_authenticate_hint`, WorkBuddy pin).  
   Plain local HTTP is **not** a warning.

2. **Attribute (`watch`)**  
   Enumerate OS TCP tables (`netstat2`), map sockets → PID → process name/path (`sysinfo`).  
   Compare against `gate.allow_process_names`. Unknown clients → **ACTIVITY ALERT** + audit `activity_alert`.

3. **Record (`serve`)**  
   Loop: scan + watch → append `mcp-guard-audit.jsonl`.

4. **Hard gate (not yet)**  
   Windows WFP / macOS pf / Linux nftables (or a userspace proxy). Only after attribution is reliable.

| Alert | When |
|-------|------|
| `exposure_alert` | Unprotected MCP surface detected |
| `activity_alert` | Non-allowlisted process is talking to the port |

Browser extensions are optional demos only — they cannot cover every client that can open loopback sockets.

## MVP status

| Capability | Status |
|------------|--------|
| `mcp-guard scan` | ✅ |
| `mcp-guard watch` | ✅ soft attribution |
| `mcp-guard serve` | ✅ scan + watch + audit |
| `mcp-guard tray` | ✅ tray + main window + agent (**default zh-CN**; `--locale en`) |
| `mcp-guard dashboard` | ✅ main window only (no tray; prefer `tray`) |
| `mcp-guard status` | ✅ menu model + audit snapshot JSON |
| `mcp-guard vault` / `vault-mcp` | ✅ NoContext secret vault |
| `mcp-guard git-scan` | ✅ Local git scan for opaque LLM reasoning blobs ([arXiv:2608.09867](https://arxiv.org/abs/2608.09867)) |
| Hard port/process block | ⏳ |
| Path / tool policy | ⏳ |

### Daily use

```bash
cargo build --release
./target/release/mcp-guard tray
```

- Auto-scan once on open  
- Minimize / close → **hide to tray**; left-click tray or “Open dashboard” to restore  
- Real exit: tray menu Quit  

Optional config: copy `mcp-guard.toml.example` → `mcp-guard.toml`.

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

### Local git: block reasoning ciphertext from landing in history

Providers wrap CoT as opaque AEAD blobs; once committed, public clones can be replay-decrypted. Case pack: [`cases/arxiv-2608-09867/`](cases/arxiv-2608-09867/).

```bash
mcp-guard git-scan .
mcp-guard git-scan --staged .
```

## Build

```bash
cargo build --release
./target/release/mcp-guard scan
./target/release/mcp-guard watch
./target/release/mcp-guard serve --once
```

```bash
RUST_LOG=debug mcp-guard scan --ports 50551,52412
```

## License

MIT
