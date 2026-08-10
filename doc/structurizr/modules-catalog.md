# Modules catalog

Target topology: **plugins → contracts (+ optional infra)**; **compose** wires implementations.

| id | role | path | allowed deps | notes |
|----|------|------|--------------|-------|
| contracts | contracts | `src/contracts.rs` | config (types in trait sigs) | Ports + DTOs; no plugin imports |
| config | infra | `src/config.rs` | — | TOML |
| cli | compose | `src/main.rs` | runtime, config, plugins (wire) | Assembles `run_with(...)` |
| runtime | compose | `src/serve.rs` | contracts, config | Dispatches via Scanner/Watcher/AlertSink |
| scan | plugin | `src/scan.rs` | contracts, config | `LoopbackScanner`; Env: loopback HTTP |
| watch | plugin | `src/watch.rs` | contracts, config | `SoftWatcher`; Env: OS TCP/PID |
| audit | plugin | `src/audit.rs` | contracts, config | `JsonlSink` |
| ui_shell | plugin | planned | contracts, ui_config | Presentation; UI as config/code |
| ui_config | infra | `ui/` | — | Theme/layout files (data) |
