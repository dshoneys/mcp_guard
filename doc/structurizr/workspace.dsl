workspace "MCP Guard" "Local agent: scan / watch / audit / (later) gate MCP tool-call surfaces. ADL authority for O(1) plugins + R_manual." {

    !identifiers hierarchical
    !adrs decisions

    model {
        operator = person "Operator" "Runs mcp-guard CLI / future tray agent; reviews alerts"

        osNet = softwareSystem "Host OS network tables" "TCP tables, process list (netstat2/sysinfo)" {
            tags "External" "Env"
        }
        mcpServers = softwareSystem "Local MCP servers" "e.g. WorkBuddy Ardot on 127.0.0.1:50551" {
            tags "External" "Env"
        }

        mcpGuard = softwareSystem "MCP Guard" "Resident host agent for MCP exposure scan, soft watch, audit; hard gate later" {

            group "L2 — composition" {
                cli = container "CLI" "clap entry: scan / watch / serve / version" "Rust bin mcp-guard" {
                    tags "Compose"
                    properties {
                        "path" "src/main.rs"
                        "role" "compose"
                        "horizon.intention" "Parse argv; load config; invoke runtime once"
                    }
                }

                runtime = container "Runtime" "Periodic serve loop: dispatch ports, write audit" "Rust" {
                    tags "Compose"
                    properties {
                        "path" "src/serve.rs (target: runtime crate)"
                        "role" "compose"
                        "horizon.intention" "Orchestrate Scanner/Watcher/AlertSink; no business rules"
                        "as-is.debt" "Currently imports scan/watch modules directly — violates target NoCross until refactor"
                    }
                }
            }

            group "L2 — contracts" {
                contracts = container "Contracts" "Traits/DTO: Scanner, Watcher, Gate, AlertSink, reports" "Rust" {
                    tags "Contracts"
                    properties {
                        "path" "src/contracts.rs"
                        "role" "contracts"
                        "horizon.intention" "Stable ports only; slow-growing |C|"
                        "horizon.out" "None to plugins"
                    }
                }
            }

            group "L2 — plugins (out-degree O(1))" {
                scanPlugin = container "Scan plugin" "Loopback HTTP probe; CORS/auth heuristics; exposure_alert" "Rust" {
                    tags "Plugin"
                    properties {
                        "path" "src/scan.rs"
                        "role" "plugin"
                        "horizon.intention" "Detect exploitable local MCP-like surfaces"
                        "horizon.deps" "contracts (+ config infra)"
                        "test_kind" "unit+env"
                    }
                }

                watchPlugin = container "Watch plugin" "TCP→PID attribution; activity_alert" "Rust" {
                    tags "Plugin"
                    properties {
                        "path" "src/watch.rs"
                        "role" "plugin"
                        "horizon.intention" "Attribute listeners/clients on watched ports"
                        "horizon.deps" "contracts (+ config infra)"
                        "test_kind" "unit+env"
                    }
                }

                auditPlugin = container "Audit sink" "JSONL AlertSink implementation" "Rust" {
                    tags "Plugin" "Infra"
                    properties {
                        "path" "src/audit.rs"
                        "role" "plugin"
                        "horizon.intention" "Persist scan/watch/alert events"
                        "horizon.deps" "contracts (+ config infra)"
                        "test_kind" "unit"
                    }
                }

                gatePlugin = container "Gate plugin" "Hard allow/deny (WFP/proxy) — planned" "Rust" {
                    tags "Plugin" "Planned"
                    properties {
                        "path" "src/gate (planned)"
                        "role" "plugin"
                        "horizon.intention" "Block unknown clients on watched ports"
                        "horizon.deps" "contracts only"
                        "test_kind" "unit+manual"
                        "status" "planned"
                    }
                }

                uiShell = container "UI shell" "Tray/window shell; reads ui config" "Rust" {
                    tags "Plugin"
                    properties {
                        "path" "src/ui_shell/"
                        "role" "plugin"
                        "horizon.intention" "Presentation only; UI as config/code"
                        "horizon.deps" "contracts + ui_config infra"
                    }
                }
            }

            group "L2 — config" {
                configMod = container "Config" "TOML load: scan/gate/serve/audit" "Rust" {
                    tags "Infra"
                    properties {
                        "path" "src/config.rs"
                        "role" "infra"
                    }
                }

                uiConfig = container "UI config files" "ui/*.toml tokens and copy — data not a plugin" "files" {
                    tags "Infra"
                    properties {
                        "path" "ui/"
                        "role" "infra"
                    }
                }
            }

            // People
            operator -> cli "runs commands"
            operator -> runtime "reads audit / alerts"

            // Compose wiring (allowed fan-in)
            cli -> runtime "serve"
            cli -> scanPlugin "scan once"
            cli -> watchPlugin "watch once"
            cli -> configMod "load"

            runtime -> contracts "dispatch via ports"
            runtime -> configMod "interval / paths"

            // Target: plugins → contracts only
            scanPlugin -> contracts "implements Scanner"
            watchPlugin -> contracts "implements Watcher"
            auditPlugin -> contracts "implements AlertSink"
            gatePlugin -> contracts "implements Gate"
            uiShell -> contracts "implements UI ports"
            uiShell -> uiConfig "reads themes/copy"

            // Env (adapters touch OS — not plugin→plugin)
            scanPlugin -> mcpServers "HTTP probe loopback" "Env"
            watchPlugin -> osNet "enumerate TCP + PID" "Env"
            gatePlugin -> osNet "enforce (planned)" "Env"
        }
    }

    views {
        systemContext mcpGuard "SystemContext" {
            include *
            autoLayout
        }

        container mcpGuard "Containers" {
            include *
            autoLayout
        }

        styles {
            element "Person" {
                shape Person
                background #08427b
                color #ffffff
            }
            element "External" {
                background #999999
                color #ffffff
            }
            element "Plugin" {
                background #1168bd
                color #ffffff
            }
            element "Contracts" {
                background #85bbf0
                color #000000
            }
            element "Compose" {
                background #2e7d32
                color #ffffff
            }
            element "Planned" {
                border dashed
                opacity 60
            }
            element "Env" {
                background #6b5b95
                color #ffffff
            }
        }
    }
}
