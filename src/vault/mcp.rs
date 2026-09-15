//! Minimal MCP stdio server for NoContext vault tools.

use crate::vault::scrub::scrub_secret;
use crate::vault::store::Vault;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::process::Command;

pub fn run_stdio_mcp(vault: &Vault) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let req: Value = serde_json::from_str(&line).context("parse mcp json")?;
        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(json!({}));

        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "mcp-guard-vault", "version": env!("CARGO_PKG_VERSION") }
            })),
            "notifications/initialized" | "initialized" => {
                // no response for notifications without id
                if id.is_null() {
                    continue;
                }
                Ok(json!({}))
            }
            "tools/list" => Ok(json!({ "tools": tool_defs() })),
            "tools/call" => handle_tools_call(vault, &params),
            "ping" => Ok(json!({})),
            _ => Err(anyhow::anyhow!("method not found: {method}")),
        };

        // notifications with null id: skip reply
        if id.is_null() && method.starts_with("notifications/") {
            continue;
        }

        let resp = match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err(err) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32000, "message": err.to_string() }
            }),
        };
        writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
        stdout.flush()?;
    }
    Ok(())
}

fn tool_defs() -> Vec<Value> {
    vec![
        json!({
            "name": "vault_list",
            "description": "List secret names and aliases in MCP Guard vault (no values).",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        }),
        json!({
            "name": "vault_issue_ref",
            "description": "Issue a short-lived opaque ref for a named secret (or alias). Never returns plaintext.",
            "inputSchema": {
                "type": "object",
                "properties": { "name": { "type": "string" } },
                "required": ["name"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "vault_ref_info",
            "description": "Check whether a vault ref is still valid (no plaintext).",
            "inputSchema": {
                "type": "object",
                "properties": { "ref": { "type": "string" } },
                "required": ["ref"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "vault_rename",
            "description": "Rename a canonical secret. Aliases that pointed at the old name are updated. No plaintext returned.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": { "type": "string" },
                    "to": { "type": "string" }
                },
                "required": ["from", "to"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "vault_alias",
            "description": "Create or update an alias → secret name (for scripts that need a specific env/token name). No plaintext.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "alias": { "type": "string" },
                    "name": { "type": "string", "description": "Canonical secret name (or existing alias)" }
                },
                "required": ["alias", "name"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "vault_unalias",
            "description": "Remove an alias (does not delete the secret).",
            "inputSchema": {
                "type": "object",
                "properties": { "alias": { "type": "string" } },
                "required": ["alias"],
                "additionalProperties": false
            }
        }),
        json!({
            "name": "vault_run_with_secret",
            "description": "Run a local command with secret(s) injected into env. Use name+env_key for one secret, or env_map {ENV_VAR: vault_name_or_alias} for several. Stdout/stderr scrubbed. Plaintext never returned.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Secret name or alias (single-secret mode)" },
                    "command": { "type": "string" },
                    "args": { "type": "array", "items": { "type": "string" } },
                    "env_key": { "type": "string", "description": "Env var name for single-secret mode (default SECRET)" },
                    "env_map": {
                        "type": "object",
                        "description": "Mapping ENV_VAR → vault secret name/alias (multi-secret mode)",
                        "additionalProperties": { "type": "string" }
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }
        }),
    ]
}

fn handle_tools_call(vault: &Vault, params: &Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing tool name"))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    // Hard ban
    if name == "vault_get" || name == "get_secret" {
        anyhow::bail!("vault_get is forbidden (NoContext); use vault_issue_ref or vault_run_with_secret");
    }

    let payload = match name {
        "vault_list" => {
            let list = vault.list()?;
            let aliases = vault.list_aliases()?;
            json!({ "secrets": list, "aliases": aliases })
        }
        "vault_issue_ref" => {
            let secret_name = args
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow::anyhow!("name required"))?;
            let r = vault.issue_ref(secret_name)?;
            json!({
                "ref": r.ref_id,
                "name": r.name,
                "expires_at_unix": r.expires_at_unix,
                "note": "Pass this ref to Guard-aware tools; plaintext is never returned over MCP."
            })
        }
        "vault_ref_info" => {
            let id = args
                .get("ref")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow::anyhow!("ref required"))?;
            vault.ref_info(id)?
        }
        "vault_rename" => {
            let from = args
                .get("from")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow::anyhow!("from required"))?;
            let to = args
                .get("to")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow::anyhow!("to required"))?;
            vault.rename(from, to)?;
            json!({ "renamed": true, "from": from, "to": to })
        }
        "vault_alias" => {
            let alias = args
                .get("alias")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow::anyhow!("alias required"))?;
            let target = args
                .get("name")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow::anyhow!("name required"))?;
            vault.set_alias(alias, target)?;
            let canonical = vault.canonical_name(alias)?;
            json!({ "alias": alias, "name": canonical })
        }
        "vault_unalias" => {
            let alias = args
                .get("alias")
                .and_then(|n| n.as_str())
                .ok_or_else(|| anyhow::anyhow!("alias required"))?;
            let removed = vault.remove_alias(alias)?;
            json!({ "alias": alias, "removed": removed })
        }
        "vault_run_with_secret" => run_with_secret(vault, &args)?,
        other => anyhow::bail!("unknown tool: {other}"),
    };

    assert_nocontext(&payload)?;

    Ok(json!({
        "content": [{ "type": "text", "text": serde_json::to_string_pretty(&payload)? }],
        "isError": false
    }))
}

fn run_with_secret(vault: &Vault, args: &Value) -> Result<Value> {
    let command = args
        .get("command")
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("command required"))?;
    let cmd_args: Vec<String> = args
        .get("args")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Build ENV_VAR → plaintext locally (never returned).
    let mut injections: Vec<(String, String, String)> = Vec::new(); // env_key, vault_name, value
    if let Some(map) = args.get("env_map").and_then(|v| v.as_object()) {
        if map.is_empty() {
            bail!("env_map must not be empty");
        }
        for (env_key, vault_name_v) in map {
            let vault_name = vault_name_v
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("env_map values must be strings"))?;
            let canonical = vault.canonical_name(vault_name)?;
            let secret = vault.resolve_local(&canonical)?;
            injections.push((env_key.clone(), canonical, secret));
        }
    } else {
        let secret_name = args
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow::anyhow!("name or env_map required"))?;
        let env_key = args
            .get("env_key")
            .and_then(|n| n.as_str())
            .unwrap_or("SECRET");
        let canonical = vault.canonical_name(secret_name)?;
        let secret = vault.resolve_local(&canonical)?;
        injections.push((env_key.to_string(), canonical, secret));
    }

    let mut cmd = Command::new(command);
    cmd.args(&cmd_args);
    for (env_key, _, secret) in &injections {
        cmd.env(env_key, secret);
    }
    let output = cmd
        .output()
        .with_context(|| format!("spawn {command}"))?;

    let mut stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    for (_, _, secret) in &injections {
        stdout = scrub_secret(&stdout, secret);
        stderr = scrub_secret(&stderr, secret);
    }

    let mapping: Vec<Value> = injections
        .iter()
        .map(|(env_key, name, _)| json!({ "env_key": env_key, "secret_name": name }))
        .collect();

    Ok(json!({
        "exit_code": output.status.code(),
        "stdout": stdout,
        "stderr": stderr,
        "mapping": mapping,
        "note": "Secret value(s) injected into env only and scrubbed from captured output."
    }))
}

fn bail_tool(msg: &str) -> Result<Value> {
    Err(anyhow::anyhow!("{msg}"))
}

/// Ensure JSON payload has no obvious secret field names with string values that look like vault dumps.
pub fn assert_nocontext(payload: &Value) -> Result<()> {
    if let Some(obj) = payload.as_object() {
        for banned in ["value", "secret", "password", "token", "plaintext", "api_key"] {
            if obj.contains_key(banned) {
                anyhow::bail!(
                    "NoContext violation: payload must not contain field '{banned}'"
                );
            }
        }
    }
    Ok(())
}

/// Build a tools/call result JSON for unit tests (no stdio).
pub fn dispatch_tool_for_test(vault: &Vault, tool: &str, arguments: Value) -> Result<Value> {
    handle_tools_call(
        vault,
        &json!({ "name": tool, "arguments": arguments }),
    )
}
