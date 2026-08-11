//! Minimal MCP stdio server for NoContext vault tools.

use crate::vault::scrub::scrub_secret;
use crate::vault::store::Vault;
use anyhow::{Context, Result};
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
            "description": "List secret names in MCP Guard vault (no values).",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
        }),
        json!({
            "name": "vault_issue_ref",
            "description": "Issue a short-lived opaque ref for a named secret. Never returns plaintext.",
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
            "name": "vault_run_with_secret",
            "description": "Run a local command with SECRET=<value> in env. Stdout/stderr are scrubbed. Plaintext never returned.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Secret name" },
                    "command": { "type": "string" },
                    "args": { "type": "array", "items": { "type": "string" } },
                    "env_key": { "type": "string", "description": "Env var name (default SECRET)" }
                },
                "required": ["name", "command"],
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
            json!({ "secrets": list })
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
    let secret_name = args
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("name required"))?;
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
    let env_key = args
        .get("env_key")
        .and_then(|n| n.as_str())
        .unwrap_or("SECRET");

    let secret = vault.resolve_local(secret_name)?;
    let output = Command::new(command)
        .args(&cmd_args)
        .env(env_key, &secret)
        .output()
        .with_context(|| format!("spawn {command}"))?;

    let stdout = scrub_secret(&String::from_utf8_lossy(&output.stdout), &secret);
    let stderr = scrub_secret(&String::from_utf8_lossy(&output.stderr), &secret);

    Ok(json!({
        "exit_code": output.status.code(),
        "stdout": stdout,
        "stderr": stderr,
        "env_key": env_key,
        "secret_name": secret_name,
        "note": "Secret value was injected into env only and scrubbed from captured output."
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
