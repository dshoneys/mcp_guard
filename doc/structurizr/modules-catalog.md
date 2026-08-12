# Modules catalog

Target topology: **plugins → contracts (+ optional infra)**; **compose** wires implementations.

| id | role | path | allowed deps | notes |
|----|------|------|--------------|-------|
| contracts | contracts | `src/contracts.rs` | config (types in trait sigs) | Ports + DTOs; no plugin imports |
| config | infra | `src/config.rs` | — | TOML |
| net_enum | infra | `src/net_enum.rs` | — | Loopback/unspecified TCP LISTEN ports |
| cli | compose | `src/main.rs` | runtime, config, plugins (wire) | Assembles `run_with(...)` |
| runtime | compose | `src/serve.rs` | contracts, config | Dispatches via Scanner/Watcher/AlertSink |
| scan | plugin | `src/scan.rs` | contracts, config, net_enum | Enumerate + HTTP warn + MCP `tools/list` probe; Env: loopback |
| git_scan | plugin | `src/git_scan.rs` | contracts, config | Local git tracked/staged scan for opaque LLM reasoning signatures |
| cases | infra | `cases/` | — | Offense/defense case packs (docs + fixtures), not a plugin |
| watch | plugin | `src/watch.rs` | contracts, config, net_enum | SoftWatcher on discovered listen ports |
| audit | plugin | `src/audit.rs` | contracts, config | `JsonlSink` |
| vault | plugin | `src/vault/` | contracts, config | NoContext secrets + `vault-mcp` |
| ui_shell | plugin | `src/ui_shell/` | contracts, ui_config, vault | Menu + tray/dashboard + i18n; plugins hub hosts vault |
| ui_config | infra | `ui/` | — | Theme/layout files (data) |
