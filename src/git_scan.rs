//! Local git scan for opaque LLM reasoning signatures (arXiv:2608.09867 defense).

use crate::config::{Config, GitScanConfig};
use crate::contracts::{GitScanFinding, GitScanReport, GitScanner};
use anyhow::{bail, Context, Result};
use chrono::Utc;
use regex::bytes::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

struct Detector {
    hint: &'static str,
    re: Regex,
}

fn detectors() -> &'static [Detector] {
    static DET: OnceLock<Vec<Detector>> = OnceLock::new();
    DET.get_or_init(|| {
        vec![
            Detector {
                hint: "anthropic_thinking_signature",
                re: Regex::new(
                    r#"(?s)"type"\s*:\s*"thinking".{0,800}?"signature"\s*:\s*"([A-Za-z0-9+/=_-]{80,})""#,
                )
                .expect("anthropic thinking regex"),
            },
            Detector {
                hint: "anthropic_thinking_signature",
                re: Regex::new(
                    r#"(?s)"signature"\s*:\s*"([A-Za-z0-9+/=_-]{80,})".{0,800}?"type"\s*:\s*"thinking""#,
                )
                .expect("anthropic thinking alt regex"),
            },
            Detector {
                hint: "openai_encrypted_content",
                re: Regex::new(r#""encrypted_content"\s*:\s*"([A-Za-z0-9+/=_-]{80,})""#)
                    .expect("openai encrypted_content regex"),
            },
            Detector {
                hint: "gemini_thought_signature",
                re: Regex::new(r#""thoughtSignature"\s*:\s*"([A-Za-z0-9+/=_-]{40,})""#)
                    .expect("gemini thoughtSignature regex"),
            },
            Detector {
                hint: "generic_signature_field",
                re: Regex::new(r#""signature"\s*:\s*"([A-Za-z0-9+/=_-]{80,})""#)
                    .expect("generic signature regex"),
            },
        ]
    })
}

/// Detect reasoning AEAD / signature blobs in raw bytes.
pub fn detect_in_bytes(data: &[u8]) -> Vec<GitScanFinding> {
    if data.is_empty() {
        return Vec::new();
    }
    // Skip obvious binaries unless they mention reasoning keys.
    if data[..data.len().min(2048)].contains(&0)
        && !data.windows(9).any(|w| w == b"signature")
        && !data.windows(18).any(|w| w == b"encrypted_content")
        && !data.windows(16).any(|w| w == b"thoughtSignature")
        && !data.windows(17).any(|w| w == b"reasoning_details")
    {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::<(String, usize)>::new();
    for det in detectors() {
        for caps in det.re.captures_iter(data) {
            let Some(m) = caps.get(1) else { continue };
            let token = m.as_bytes();
            let min_len = if det.hint == "gemini_thought_signature" {
                40
            } else {
                80
            };
            if token.len() < min_len {
                continue;
            }
            let offset = m.start();
            let key = (det.hint.to_string(), offset);
            if !seen.insert(key) {
                continue;
            }
            let preview_raw = String::from_utf8_lossy(token);
            let preview = if preview_raw.chars().count() > 48 {
                let mut s: String = preview_raw.chars().take(48).collect();
                s.push('…');
                s
            } else {
                preview_raw.into_owned()
            };
            out.push(GitScanFinding {
                provider_hint: det.hint.to_string(),
                path: String::new(),
                offset,
                token_len: token.len(),
                preview,
            });
        }
    }
    out
}

fn git_exe() -> String {
    which("hutao").unwrap_or_else(|| "git".into())
}

fn which(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cand = dir.join(name);
        if cand.is_file() {
            return Some(cand.to_string_lossy().into_owned());
        }
        #[cfg(windows)]
        {
            let cand_cmd = dir.join(format!("{name}.cmd"));
            if cand_cmd.is_file() {
                return Some(cand_cmd.to_string_lossy().into_owned());
            }
            let cand_exe = dir.join(format!("{name}.exe"));
            if cand_exe.is_file() {
                return Some(cand_exe.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn git_output(root: &Path, args: &[&str]) -> Result<String> {
    let exe = git_exe();
    let out = Command::new(&exe)
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("spawn {exe}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("git {:?} failed: {}", args, err.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn list_targets(root: &Path, staged_only: bool) -> Result<Vec<PathBuf>> {
    let stdout = if staged_only {
        git_output(
            root,
            &["diff", "--cached", "--name-only", "--diff-filter=ACMR"],
        )?
    } else {
        git_output(root, &["ls-files", "-z"])?
    };
    let paths: Vec<PathBuf> = if staged_only {
        stdout
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect()
    } else {
        stdout
            .split('\0')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect()
    };
    Ok(paths)
}

fn path_excluded(rel: &Path, cfg: &GitScanConfig) -> bool {
    let s = rel.to_string_lossy().replace('\\', "/");
    cfg.exclude_substrings.iter().any(|ex| s.contains(ex))
}

fn extension_allowed(path: &Path, cfg: &GitScanConfig) -> bool {
    if cfg.extensions.is_empty() {
        return true;
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // agent-messages.jsonl style
    if name == "agent-messages.jsonl" {
        return true;
    }
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|e| format!(".{}", e.to_ascii_lowercase()))
        .unwrap_or_default();
    cfg.extensions
        .iter()
        .any(|want| want.eq_ignore_ascii_case(&ext))
}

/// Scan a local git working tree (tracked or staged files).
pub fn scan_repo(cfg: &Config, root: &Path, staged_only: bool) -> Result<GitScanReport> {
    let root = root
        .canonicalize()
        .with_context(|| format!("canonicalize {}", root.display()))?;
    // Ensure it is a git repo
    let _ = git_output(&root, &["rev-parse", "--is-inside-work-tree"])?;

    let targets = list_targets(&root, staged_only)?;
    let mut findings = Vec::new();
    let mut files_scanned = 0usize;

    for rel in targets {
        if path_excluded(&rel, &cfg.git_scan) {
            continue;
        }
        if !extension_allowed(&rel, &cfg.git_scan) {
            continue;
        }
        let abs = root.join(&rel);
        let meta = match std::fs::metadata(&abs) {
            Ok(m) => m,
            Err(_) => continue, // deleted / missing
        };
        if !meta.is_file() || meta.len() == 0 || meta.len() > cfg.git_scan.max_file_bytes {
            continue;
        }
        let data = match std::fs::read(&abs) {
            Ok(d) => d,
            Err(_) => continue,
        };
        files_scanned += 1;
        for mut f in detect_in_bytes(&data) {
            f.path = rel.to_string_lossy().replace('\\', "/");
            findings.push(f);
        }
    }

    Ok(GitScanReport {
        root: root.display().to_string(),
        mode: if staged_only {
            "staged".into()
        } else {
            "tracked".into()
        },
        scanned_at: Utc::now().to_rfc3339(),
        files_scanned,
        findings,
    })
}

pub struct LocalGitScanner;

impl GitScanner for LocalGitScanner {
    fn scan_repo(
        &self,
        cfg: &Config,
        root: &Path,
        staged_only: bool,
    ) -> Result<GitScanReport> {
        scan_repo(cfg, root, staged_only)
    }
}
