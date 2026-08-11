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

## MCP tools (v1)

| Tool | Returns to model | Notes |
|------|------------------|-------|
| `vault_list` | id, name, updated_at | OK |
| `vault_issue_ref` | ref, name, expires_at | OK |
| `vault_ref_info` | name, expires, valid | OK |
| `vault_run_with_secret` | exit_code, scrubbed stdout/stderr | Scrub occurrences of secret |
| ~~`vault_get`~~ | — | **Forbidden** |

## Storage

- Encrypted blob on disk (`aes-gcm`); key file local to user profile / config dir.
- UI/CLI can write secrets; UI never echoes stored values after save.

## Tests

- Unit: encrypt/decrypt roundtrip; scrubber; ref expiry.
- Contract: MCP tool schemas reject get; list/issue_ref contain no value field.
- Manual: wire Cursor MCP to `mcp-guard vault-mcp`; confirm tool payloads have no plaintext.
