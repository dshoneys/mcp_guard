//! AES-GCM encrypted secret store + short-lived opaque refs.

use crate::config::VaultConfig;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncRecord {
    nonce_b64: String,
    ct_b64: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct VaultFile {
    version: u32,
    secrets: HashMap<String, EncRecord>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SecretMeta {
    pub name: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VaultRef {
    pub ref_id: String,
    pub name: String,
    pub expires_at_unix: u64,
}

struct LiveRef {
    name: String,
    expires_at: SystemTime,
}

pub struct Vault {
    cfg: VaultConfig,
    key: [u8; KEY_LEN],
    refs: Mutex<HashMap<String, LiveRef>>,
}

impl Vault {
    pub fn open(cfg: &VaultConfig) -> Result<Self> {
        let key = load_or_create_key(&cfg.key_path)?;
        if let Some(parent) = cfg.store_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        if !cfg.store_path.exists() {
            let empty = VaultFile {
                version: 1,
                secrets: HashMap::new(),
            };
            write_store(&cfg.store_path, &empty)?;
        }
        Ok(Self {
            cfg: cfg.clone(),
            key,
            refs: Mutex::new(HashMap::new()),
        })
    }

    fn read(&self) -> Result<VaultFile> {
        read_store(&self.cfg.store_path)
    }

    fn write(&self, file: &VaultFile) -> Result<()> {
        write_store(&self.cfg.store_path, file)
    }

    pub fn list(&self) -> Result<Vec<SecretMeta>> {
        let file = self.read()?;
        let mut out: Vec<_> = file
            .secrets
            .iter()
            .map(|(name, rec)| SecretMeta {
                name: name.clone(),
                updated_at: rec.updated_at.clone(),
            })
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    pub fn put(&self, name: &str, value: &str) -> Result<()> {
        if name.trim().is_empty() {
            bail!("secret name required");
        }
        if value.is_empty() {
            bail!("secret value required");
        }
        let mut file = self.read()?;
        let (nonce, ct) = encrypt(&self.key, value.as_bytes())?;
        file.secrets.insert(
            name.to_string(),
            EncRecord {
                nonce_b64: B64.encode(nonce),
                ct_b64: B64.encode(ct),
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
        );
        self.write(&file)
    }

    pub fn delete(&self, name: &str) -> Result<bool> {
        let mut file = self.read()?;
        let removed = file.secrets.remove(name).is_some();
        if removed {
            self.write(&file)?;
        }
        Ok(removed)
    }

    /// Local-only plaintext resolve (never an MCP tool result).
    pub fn resolve_local(&self, name: &str) -> Result<String> {
        let file = self.read()?;
        let Some(rec) = file.secrets.get(name) else {
            bail!("secret not found: {name}");
        };
        let nonce = B64.decode(&rec.nonce_b64).context("nonce b64")?;
        let ct = B64.decode(&rec.ct_b64).context("ct b64")?;
        let pt = decrypt(&self.key, &nonce, &ct)?;
        String::from_utf8(pt).context("secret utf8")
    }

    pub fn issue_ref(&self, name: &str) -> Result<VaultRef> {
        // Ensure exists
        let _ = self.resolve_local(name)?;
        let mut id = [0u8; 16];
        rand::rng().fill(&mut id);
        let ref_id = format!("vr_{}", hex::encode(id));
        let ttl = Duration::from_secs(self.cfg.ref_ttl_secs.max(30));
        let expires_at = SystemTime::now() + ttl;
        let expires_at_unix = expires_at
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.refs.lock().unwrap().insert(
            ref_id.clone(),
            LiveRef {
                name: name.to_string(),
                expires_at,
            },
        );
        Ok(VaultRef {
            ref_id,
            name: name.to_string(),
            expires_at_unix,
        })
    }

    pub fn ref_info(&self, ref_id: &str) -> Result<serde_json::Value> {
        let guard = self.refs.lock().unwrap();
        match guard.get(ref_id) {
            Some(r) if r.expires_at > SystemTime::now() => Ok(serde_json::json!({
                "ref": ref_id,
                "name": r.name,
                "valid": true,
                "expires_at_unix": r.expires_at.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
            })),
            Some(_) => Ok(serde_json::json!({
                "ref": ref_id,
                "valid": false,
                "reason": "expired",
            })),
            None => Ok(serde_json::json!({
                "ref": ref_id,
                "valid": false,
                "reason": "unknown",
            })),
        }
    }

    pub fn resolve_ref_local(&self, ref_id: &str) -> Result<(String, String)> {
        let (name, expired) = {
            let guard = self.refs.lock().unwrap();
            match guard.get(ref_id) {
                Some(r) if r.expires_at > SystemTime::now() => (r.name.clone(), false),
                Some(_) => (String::new(), true),
                None => bail!("unknown ref"),
            }
        };
        if expired {
            bail!("ref expired");
        }
        let value = self.resolve_local(&name)?;
        Ok((name, value))
    }
}

fn load_or_create_key(path: &Path) -> Result<[u8; KEY_LEN]> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    if path.exists() {
        let raw = fs::read(path).with_context(|| format!("read key {}", path.display()))?;
        if raw.len() != KEY_LEN {
            bail!("vault key must be {KEY_LEN} bytes");
        }
        let mut key = [0u8; KEY_LEN];
        key.copy_from_slice(&raw);
        return Ok(key);
    }
    let mut key = [0u8; KEY_LEN];
    rand::rng().fill(&mut key);
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .with_context(|| format!("create key {}", path.display()))?;
    f.write_all(&key)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(key)
}

fn read_store(path: &Path) -> Result<VaultFile> {
    let raw = fs::read_to_string(path).with_context(|| format!("read vault {}", path.display()))?;
    Ok(serde_json::from_str(&raw)?)
}

fn write_store(path: &Path, file: &VaultFile) -> Result<()> {
    let raw = serde_json::to_string_pretty(file)?;
    fs::write(path, raw).with_context(|| format!("write vault {}", path.display()))?;
    Ok(())
}

fn encrypt(key: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<([u8; NONCE_LEN], Vec<u8>)> {
    let cipher = Aes256Gcm::new_from_slice(key).context("aes key")?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rng().fill(&mut nonce_bytes);
    let nonce = Nonce::try_from(&nonce_bytes[..]).map_err(|_| anyhow::anyhow!("nonce"))?;
    let ct = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|e| anyhow::anyhow!("encrypt: {e}"))?;
    Ok((nonce_bytes, ct))
}

fn decrypt(key: &[u8; KEY_LEN], nonce: &[u8], ct: &[u8]) -> Result<Vec<u8>> {
    if nonce.len() != NONCE_LEN {
        bail!("bad nonce len");
    }
    let cipher = Aes256Gcm::new_from_slice(key).context("aes key")?;
    let nonce = Nonce::try_from(nonce).map_err(|_| anyhow::anyhow!("nonce"))?;
    cipher
        .decrypt(&nonce, ct)
        .map_err(|e| anyhow::anyhow!("decrypt: {e}"))
}
