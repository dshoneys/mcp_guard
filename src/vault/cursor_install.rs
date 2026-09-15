//! Detect local Cursor and install `vault-mcp` into `~/.cursor/mcp.json`.
//!
//! See `doc/ux/REQ-VAULT-MCP-UI/` and `doc/structurizr/VAULT-NOCONTEXT.md`.

use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

/// Stable key under `mcpServers` (do not rename casually).
pub const SERVER_KEY: &str = "mcp-guard-vault";

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CursorMcpState {
    NoCursor,
    NotInstalled,
    Installed,
    Broken,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CursorMcpStatus {
    pub state: CursorMcpState,
    pub detail: String,
    pub mcp_json_path: String,
    pub cursor_detected: bool,
    pub exe: String,
}

/// User-global Cursor MCP config (`~/.cursor/mcp.json`).
pub fn default_mcp_json_path() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".cursor")
        .join("mcp.json")
}

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Best-effort: Cursor app or `~/.cursor` config dir.
pub fn cursor_detected() -> bool {
    cursor_detected_with_home(home_dir().as_deref())
}

pub fn cursor_detected_with_home(home: Option<&Path>) -> bool {
    if let Some(home) = home {
        if home.join(".cursor").is_dir() {
            return true;
        }
    }
    #[cfg(target_os = "macos")]
    {
        if Path::new("/Applications/Cursor.app").is_dir() {
            return true;
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(home) = home {
            let local = home
                .join("AppData")
                .join("Local")
                .join("Programs")
                .join("cursor");
            if local.is_dir() {
                return true;
            }
        }
    }
    false
}

/// Probe status for UI strip.
pub fn probe(exe: &Path, mcp_json: &Path, cursor_ok: bool) -> CursorMcpStatus {
    let exe_s = exe.display().to_string();
    let path_s = mcp_json.display().to_string();
    if !cursor_ok {
        return CursorMcpStatus {
            state: CursorMcpState::NoCursor,
            detail: "Cursor not detected on this machine".into(),
            mcp_json_path: path_s,
            cursor_detected: false,
            exe: exe_s,
        };
    }
    match read_server_entry(mcp_json) {
        Ok(None) => CursorMcpStatus {
            state: CursorMcpState::NotInstalled,
            detail: format!("Missing `{SERVER_KEY}` in mcp.json"),
            mcp_json_path: path_s,
            cursor_detected: true,
            exe: exe_s,
        },
        Ok(Some(entry)) => match classify_entry(&entry, exe) {
            EntryClass::Ok => CursorMcpStatus {
                state: CursorMcpState::Installed,
                detail: format!("`{SERVER_KEY}` → vault-mcp"),
                mcp_json_path: path_s,
                cursor_detected: true,
                exe: exe_s,
            },
            EntryClass::Broken(reason) => CursorMcpStatus {
                state: CursorMcpState::Broken,
                detail: reason,
                mcp_json_path: path_s,
                cursor_detected: true,
                exe: exe_s,
            },
        },
        Err(err) => CursorMcpStatus {
            state: CursorMcpState::Broken,
            detail: format!("Cannot read mcp.json: {err}"),
            mcp_json_path: path_s,
            cursor_detected: true,
            exe: exe_s,
        },
    }
}

/// Merge / repair `mcp-guard-vault` without clobbering other servers.
pub fn install(exe: &Path, mcp_json: &Path) -> Result<CursorMcpStatus> {
    let exe = exe
        .canonicalize()
        .with_context(|| format!("resolve mcp-guard executable {}", exe.display()))?;
    if let Some(parent) = mcp_json.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create {}", parent.display()))?;
    }

    let mut root = if mcp_json.is_file() {
        let raw = fs::read_to_string(mcp_json)
            .with_context(|| format!("read {}", mcp_json.display()))?;
        if raw.trim().is_empty() {
            json!({ "mcpServers": {} })
        } else {
            serde_json::from_str(&raw)
                .with_context(|| format!("parse {}", mcp_json.display()))?
        }
    } else {
        json!({ "mcpServers": {} })
    };

    let servers = root
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("mcp.json root must be an object"))?
        .entry("mcpServers")
        .or_insert_with(|| json!({}));

    if !servers.is_object() {
        bail!("mcp.json mcpServers must be an object");
    }
    let cfg_path = write_vault_mcp_config(&exe)?;
    let map = servers.as_object_mut().expect("object");
    map.insert(
        SERVER_KEY.to_string(),
        json!({
            "command": exe.to_string_lossy(),
            "args": ["--config", cfg_path.to_string_lossy(), "vault-mcp"]
        }),
    );

    let pretty = serde_json::to_string_pretty(&root)?;
    fs::write(mcp_json, pretty + "\n")
        .with_context(|| format!("write {}", mcp_json.display()))?;

    Ok(probe(&exe, mcp_json, true))
}

/// Convenience: probe using default paths + current process exe.
pub fn probe_default() -> Result<CursorMcpStatus> {
    let exe = std::env::current_exe().context("current_exe")?;
    Ok(probe(
        &exe,
        &default_mcp_json_path(),
        cursor_detected(),
    ))
}

/// Convenience: install using default paths + current process exe.
pub fn install_default() -> Result<CursorMcpStatus> {
    let exe = std::env::current_exe().context("current_exe")?;
    install(&exe, &default_mcp_json_path())
}

enum EntryClass {
    Ok,
    Broken(String),
}

fn read_server_entry(mcp_json: &Path) -> Result<Option<Value>> {
    if !mcp_json.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(mcp_json)?;
    if raw.trim().is_empty() {
        return Ok(None);
    }
    let root: Value = serde_json::from_str(&raw)?;
    let Some(servers) = root.get("mcpServers").and_then(|v| v.as_object()) else {
        return Ok(None);
    };
    Ok(servers.get(SERVER_KEY).cloned())
}

fn classify_entry(entry: &Value, _exe: &Path) -> EntryClass {
    let Some(obj) = entry.as_object() else {
        return EntryClass::Broken("server entry is not an object".into());
    };
    let Some(cmd) = obj.get("command").and_then(|v| v.as_str()) else {
        return EntryClass::Broken("missing command".into());
    };
    let args_ok = obj
        .get("args")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().any(|x| x.as_str() == Some("vault-mcp")))
        .unwrap_or(false);
    if !args_ok {
        return EntryClass::Broken("args must include vault-mcp".into());
    }
    let cmd_path = PathBuf::from(cmd);
    if !cmd_path.exists() {
        return EntryClass::Broken(format!("command not found: {cmd}"));
    }
    // Path may differ from current exe (older install); still OK if file exists + args.
    EntryClass::Ok
}

/// Prefer repo root when running from `target/debug|release/mcp-guard`.
fn install_cwd(exe: &Path) -> PathBuf {
    if let Some(parent) = exe.parent() {
        let name = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if matches!(name, "debug" | "release") {
            if let Some(target) = parent.parent() {
                if target.file_name().and_then(|s| s.to_str()) == Some("target") {
                    if let Some(root) = target.parent() {
                        return root.to_path_buf();
                    }
                }
            }
        }
        return parent.to_path_buf();
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Absolute vault paths so Cursor-spawned vault-mcp does not depend on cwd.
fn write_vault_mcp_config(exe: &Path) -> Result<PathBuf> {
    let root = install_cwd(exe);
    let cfg_path = root.join("mcp-guard.vault-mcp.toml");
    let store = root.join("mcp-guard-vault.enc");
    let key = root.join("mcp-guard-vault.key");
    let body = format!(
        "[vault]\nstore_path = \"{}\"\nkey_path = \"{}\"\nref_ttl_secs = 300\n",
        store.display(),
        key.display()
    );
    fs::write(&cfg_path, body).with_context(|| format!("write {}", cfg_path.display()))?;
    Ok(cfg_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(tag: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("mcp-guard-cursor-{tag}-{n}"))
    }

    #[test]
    fn install_merges_without_clobber() {
        let dir = tmp("merge");
        fs::create_dir_all(&dir).unwrap();
        let mcp = dir.join("mcp.json");
        fs::write(
            &mcp,
            r#"{"mcpServers":{"other":{"command":"echo","args":["hi"]}}}"#,
        )
        .unwrap();
        let fake_exe = dir.join("mcp-guard");
        fs::write(&fake_exe, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_exe).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_exe, perms).unwrap();
        }

        let status = install(&fake_exe, &mcp).unwrap();
        assert_eq!(status.state, CursorMcpState::Installed);
        let root: Value = serde_json::from_str(&fs::read_to_string(&mcp).unwrap()).unwrap();
        assert!(root["mcpServers"]["other"].is_object());
        assert_eq!(
            root["mcpServers"][SERVER_KEY]["args"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_str() == Some("vault-mcp")),
            true
        );
        assert!(root["mcpServers"][SERVER_KEY]["args"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str() == Some("--config")));
    }

    #[test]
    fn probe_not_installed_when_missing_key() {
        let dir = tmp("miss");
        fs::create_dir_all(&dir).unwrap();
        let mcp = dir.join("mcp.json");
        fs::write(&mcp, r#"{"mcpServers":{}}"#).unwrap();
        let exe = dir.join("mcp-guard");
        fs::write(&exe, b"x").unwrap();
        let st = probe(&exe, &mcp, true);
        assert_eq!(st.state, CursorMcpState::NotInstalled);
    }

    #[test]
    fn probe_no_cursor() {
        let dir = tmp("noc");
        let st = probe(&dir.join("x"), &dir.join("mcp.json"), false);
        assert_eq!(st.state, CursorMcpState::NoCursor);
    }

    #[test]
    fn probe_broken_missing_args() {
        let dir = tmp("brk");
        fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("mcp-guard");
        fs::write(&exe, b"x").unwrap();
        let mcp = dir.join("mcp.json");
        let body = json!({
            "mcpServers": {
                SERVER_KEY: { "command": exe.to_string_lossy(), "args": [] }
            }
        });
        fs::write(&mcp, body.to_string()).unwrap();
        let st = probe(&exe, &mcp, true);
        assert_eq!(st.state, CursorMcpState::Broken);
    }
}
