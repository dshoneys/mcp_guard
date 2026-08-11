# Loopback listen enumeration (REQ-SCAN-ENUMERATE)

## Problem

Fixed port allowlists (`50551`, `52412`, …) miss vendors that **move or hide** MCP listeners.

## Decision

Default scan/watch target set = **all TCP `LISTEN` sockets bound to loopback or unspecified** (`127.0.0.1`, `::1`, `0.0.0.0`, `::`), not a static whitelist.

- `scan.discover_listeners = true` (default)
- `scan.ports` = optional **extra pins** (always probed even if currently closed)
- `scan.max_probe_ports` = safety cap (default 512)

## Risk classification

Bare TCP open ≠ exposure (would flood UI). Exposure flags require HTTP heuristics (`cors_star`, MCP-like body/server, known ARDOT port, etc.).

## Paths

- Discover: `src/net_enum.rs`
- Probe: `src/scan.rs`
- Soft watch peers: `src/watch.rs` (same port set)
