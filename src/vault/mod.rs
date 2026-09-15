//! Encrypted secret vault + NoContext MCP surface.
//!
//! See `doc/structurizr/VAULT-NOCONTEXT.md`.

pub mod cursor_install;
pub mod mcp;
mod scrub;
mod store;

pub use cursor_install::{
    cursor_detected, default_mcp_json_path, install as install_cursor_vault_mcp,
    install_default as install_cursor_vault_mcp_default, probe as probe_cursor_vault_mcp,
    probe_default as probe_cursor_vault_mcp_default, CursorMcpState, CursorMcpStatus, SERVER_KEY,
};
pub use mcp::run_stdio_mcp;
pub use scrub::scrub_secret;
pub use store::{AliasMeta, SecretMeta, Vault, VaultRef};
