# Vault / NoContext secrets

## Problem

If an MCP tool returns a secret as a normal tool result, **hosts put that result into the model context**. “Access via MCP” and “never enter context” cannot both mean `tools/call → plaintext`.

## Decision

MCP Guard vault uses a **NoContext** contract:

1. **Never** expose plaintext in MCP tool results.
2. MCP may return: secret **names**, **opaque refs** (`vr_…`), validity, redacted run status.
3. Plaintext exists only inside the vault plugin process memory / encrypted-at-rest store.
4. Consumption paths that need plaintext are **local side-effects**:
   - `vault.run_with_secret` — spawn command with env injection; scrub secret from captured stdout/stderr before returning to the host.
   - Future: gate/proxy resolves refs into outbound requests without echoing values to the LLM.

## MCP tools (v1+)

| Tool | Returns to model | Notes |
|------|------------------|-------|
| `vault_list` | secrets + aliases (names only) | OK |
| `vault_issue_ref` | ref, name, expires_at | name may be alias → canonical |
| `vault_ref_info` | name, expires, valid | OK |
| `vault_rename` | from/to | rename canonical; rewrite aliases |
| `vault_alias` / `vault_unalias` | alias ↔ name | persistent alias for script env names |
| `vault_run_with_secret` | exit_code, scrubbed stdout/stderr, mapping | `name`+`env_key` **or** `env_map` `{ENV: vault_name}` |
| ~~`vault_get`~~ | — | **Forbidden** |

### Naming vs injection

- **Alias**: stable second name for a stored secret (e.g. `GITLAB_TOKEN` → `GITLIB_TOKEN`).
- **Rename**: change the canonical key.
- **env_map** (use-time): one-shot mapping when running a command, without changing storage.

## Storage

- Encrypted blob on disk (`aes-gcm`); key file local to user profile / config dir.
- UI/CLI can write secrets; UI never echoes stored values after save.

## Tests

- Unit: encrypt/decrypt roundtrip; scrubber; ref expiry.
- Contract: MCP tool schemas reject get; list/issue_ref contain no value field.
- Manual: wire Cursor MCP to `mcp-guard vault-mcp`; confirm tool payloads have no plaintext.

## Cursor install (dashboard)

Operator-facing detect + one-click merge into `~/.cursor/mcp.json` is **REQ-VAULT-MCP-UI** (UX under `doc/ux/REQ-VAULT-MCP-UI/`). Server key: `mcp-guard-vault` → `mcp-guard vault-mcp`. Must not clobber other `mcpServers` entries; must never return plaintext secrets.
