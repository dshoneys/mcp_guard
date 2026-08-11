//! Encrypted secret vault + NoContext MCP surface.
//!
//! See `doc/structurizr/VAULT-NOCONTEXT.md`.

pub mod mcp;
mod scrub;
mod store;

pub use mcp::run_stdio_mcp;
pub use scrub::scrub_secret;
pub use store::{SecretMeta, Vault, VaultRef};
