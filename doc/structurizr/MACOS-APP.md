# macOS app build & packaging

How to build `mcp-guard` on macOS and ship a double-clickable **MCP Guard.app** (menu-bar agent + dashboard). CLI `mcp-guard tray` remains the primary developer entry.

## Build (release)

Apple CLT may ship a bleeding-edge SDK (e.g. `MacOSX27.sdk`) whose TBD files break `ld` (`unknown architecture arm64e.x1-*`). Prefer a stable SDK:

```bash
# List SDKs
ls /Library/Developer/CommandLineTools/SDKs/

# Example that works on recent CLT installs:
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX15.4.sdk
cargo build --release
```

Install the CLI on `PATH` (optional):

```bash
cp -f target/release/mcp-guard ~/.cargo/bin/mcp-guard
```

One-shot pack + install:

```bash
./scripts/macos_pack_app.sh
# → /Applications/MCP Guard.app  (also ~/Applications, Desktop)
```

## App bundle layout

```text
MCP Guard.app/
  Contents/
    Info.plist          # CFBundleExecutable=MCPGuard, LSUIElement=true
    MacOS/
      MCPGuard          # = release mcp-guard binary (same path = same process image)
    Resources/
      mcp-guard.toml    # absolute vault paths recommended
      AppIcon.icns
```

### Rules that matter

| Rule | Why |
|------|-----|
| **CFBundleExecutable must be the Rust binary** | A shell/`exec` into `~/.cargo/bin/mcp-guard` drops bundle identity → Dock tile named **exec**, tray flaky |
| **`LSUIElement=true`** | Menu-bar agent; no Dock tile for the app |
| **Early `Accessory` policy** | `main` sets activation policy before AppKit/tao so tray/dashboard children never register a Dock tile |
| **No-arg `.app` launch → `tray`** | Finder double-click has no argv; binary injects `--config Resources/mcp-guard.toml tray` |
| **Vault paths absolute** | Default vault files are cwd-relative (`mcp-guard-vault.enc`). Wrong cwd → empty vault |
| **Tray icon = full-color logo** | Do **not** mark the brand PNG as an NSImage template; opaque dark plate → solid white menu-bar square |

## Vault config for the app

`scripts/macos_pack_app.sh` writes `Contents/Resources/mcp-guard.toml` with absolute `store_path` / `key_path` pointing at the repo (or `MCP_GUARD_VAULT_DIR`).

Example:

```toml
[vault]
store_path = "/Users/you/Documents/mcp_guard/mcp-guard-vault.enc"
key_path = "/Users/you/Documents/mcp_guard/mcp-guard-vault.key"
ref_ttl_secs = 300
```

Cursor `vault-mcp` should use the same paths (see `mcp-guard.vault-mcp.toml` / REQ-VAULT-MCP-UI).

## Run

- Double-click **MCP Guard.app**, or `open -a "MCP Guard"`
- Dev: `mcp-guard tray` / `./target/release/mcp-guard tray` from a known cwd (or `--config …`)
- Quit: tray menu **退出** only (window close hides to tray)

## Ad-hoc sign (local)

```bash
codesign --force --deep --sign - "/Applications/MCP Guard.app"
```

Notarization / Developer ID is out of scope for this beta path.
