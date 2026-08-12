#!/usr/bin/env bash
# pre-commit: block opaque LLM reasoning signatures from entering git.
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
BIN="${MCP_GUARD_BIN:-mcp-guard}"
if ! command -v "$BIN" >/dev/null 2>&1; then
  if [[ -x "$ROOT/target/release/mcp-guard" ]]; then
    BIN="$ROOT/target/release/mcp-guard"
  elif [[ -x "$ROOT/target/debug/mcp-guard" ]]; then
    BIN="$ROOT/target/debug/mcp-guard"
  else
    echo "mcp-guard not found; set MCP_GUARD_BIN or build the binary" >&2
    exit 1
  fi
fi
exec "$BIN" git-scan --staged "$ROOT"
