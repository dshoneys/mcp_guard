#!/usr/bin/env python3
"""
Reproduce the *git scrape* half of arXiv:2608.09867
  "Stealing Reasoning Traces from Proprietary LLM APIs"

Paper claim (Section 4.1): developers publish agent/session logs that still
contain opaque AEAD reasoning envelopes. Authors collected 6,708 public
trajectories (GitHub + Hugging Face) and parsed 315,320 signed blocks.

This tool ONLY reproduces discovery / inventory of those blocks in a git
object DB. It does NOT call provider APIs or implement the cross-model
"fuzzy decoder" jailbreak (Appendix C). As of Aug 2026 the paper notes
vendors patched that path after disclosure.

Patterns (public API shapes + paper wording):
  - Anthropic: content block type=thinking + signature
               type=redacted_thinking + data
  - OpenAI:    encrypted_content (Responses / reasoning items)
  - Gemini:    thoughtSignature (common client field name)

Usage:
  python scan_reasoning_blobs.py demo
  python scan_reasoning_blobs.py scan [repo]
  python scan_reasoning_blobs.py scan [repo] --json
"""

from __future__ import annotations

import argparse
import base64
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
from collections import Counter
from dataclasses import asdict, dataclass
from pathlib import Path

# --- detection patterns -----------------------------------------------------

# Claude Messages API thinking / redacted_thinking (docs + paper Fig.1)
ANTHROPIC_THINKING = re.compile(
    rb'"type"\s*:\s*"thinking"[^{}]{0,400}?"signature"\s*:\s*"([A-Za-z0-9+/=_-]{80,})"',
    re.S,
)
ANTHROPIC_THINKING_ALT = re.compile(
    rb'"signature"\s*:\s*"([A-Za-z0-9+/=_-]{80,})"[^{}]{0,400}?"type"\s*:\s*"thinking"',
    re.S,
)
ANTHROPIC_REDACTED = re.compile(
    rb'"type"\s*:\s*"redacted_thinking"[^{}]{0,200}?"data"\s*:\s*"([A-Za-z0-9+/=_-]{80,})"',
    re.S,
)
ANTHROPIC_REDACTED_ALT = re.compile(
    rb'"data"\s*:\s*"([A-Za-z0-9+/=_-]{80,})"[^{}]{0,200}?"type"\s*:\s*"redacted_thinking"',
    re.S,
)

# Paper Appendix C.2: GPT encrypted_content
OPENAI_ENC = re.compile(
    rb'"encrypted_content"\s*:\s*"([A-Za-z0-9+/=_-]{80,})"',
)

# Gemini client payloads often use thoughtSignature
GEMINI_SIG = re.compile(
    rb'"thoughtSignature"\s*:\s*"([A-Za-z0-9+/=_-]{40,})"',
)
# Some traces nest under thought / parts
GEMINI_THOUGHT = re.compile(
    rb'"thought"\s*:\s*true[^{}\]]{0,400}?"(?:inlineData|data|signature)"\s*:\s*"([A-Za-z0-9+/=_-]{80,})"',
    re.S,
)

# Standalone long signature fields (ClawBench / OpenRouter / Hermes wrap
# Anthropic envelopes inside reasoning_details JSON-as-string).
GENERIC_SIGNATURE = re.compile(
    rb'"signature"\s*:\s*"([A-Za-z0-9+/=_-]{80,})"',
)
REASONING_DETAILS = re.compile(
    rb'"reasoning_details"\s*:\s*"((?:\\.|[^"\\]){80,})"',
)

PATTERNS: list[tuple[str, re.Pattern[bytes]]] = [
    ("anthropic_thinking_signature", ANTHROPIC_THINKING),
    ("anthropic_thinking_signature", ANTHROPIC_THINKING_ALT),
    ("anthropic_redacted_thinking", ANTHROPIC_REDACTED),
    ("anthropic_redacted_thinking", ANTHROPIC_REDACTED_ALT),
    ("openai_encrypted_content", OPENAI_ENC),
    ("gemini_thought_signature", GEMINI_SIG),
    ("gemini_thought_payload", GEMINI_THOUGHT),
    ("generic_signature_field", GENERIC_SIGNATURE),
]


@dataclass
class Finding:
    provider_hint: str
    blob: str
    paths: list[str]
    size: int
    offset: int
    token_len: int
    entropy: float
    preview: str


def shannon_entropy(data: bytes) -> float:
    if not data:
        return 0.0
    n = len(data)
    counts = Counter(data)
    return -sum((c / n) * math.log2(c / n) for c in counts.values())


def git_exe() -> str:
    return shutil.which("hutao") or "git"


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [git_exe(), "-C", str(repo), *args],
        check=check,
        text=True,
        capture_output=True,
    )


def list_blobs(repo: Path) -> list[tuple[str, int]]:
    out = git(repo, "rev-list", "--objects", "--all").stdout.splitlines()
    shas = [line.split()[0] for line in out if line.strip()]
    if not shas:
        return []
    proc = subprocess.run(
        [
            git_exe(),
            "-C",
            str(repo),
            "cat-file",
            "--batch-check=%(objectname) %(objecttype) %(objectsize)",
        ],
        input="\n".join(shas) + "\n",
        text=True,
        capture_output=True,
        check=True,
    )
    blobs: list[tuple[str, int]] = []
    for line in proc.stdout.splitlines():
        parts = line.split()
        if len(parts) >= 3 and parts[1] == "blob":
            blobs.append((parts[0], int(parts[2])))
    return blobs


def read_blob(repo: Path, sha: str) -> bytes:
    return subprocess.run(
        [git_exe(), "-C", str(repo), "cat-file", "blob", sha],
        check=True,
        capture_output=True,
    ).stdout


def token_entropy(token: str) -> float:
    try:
        pad = "=" * ((4 - len(token) % 4) % 4)
        raw = base64.urlsafe_b64decode(token + pad)
    except Exception:
        try:
            raw = base64.b64decode(token + "=" * ((4 - len(token) % 4) % 4))
        except Exception:
            raw = token.encode("utf-8", errors="ignore")
    return shannon_entropy(raw)


def scan_blob(sha: str, paths: list[str], data: bytes) -> list[Finding]:
    # Skip obvious binaries unless they mention reasoning keys
    if b"\0" in data[:2048] and not any(
        k in data
        for k in (
            b"signature",
            b"encrypted_content",
            b"thoughtSignature",
            b"redacted_thinking",
            b"reasoning_details",
        )
    ):
        return []

    findings: list[Finding] = []
    seen: set[tuple[str, int]] = set()

    def add(hint: str, token: str, offset: int) -> None:
        key = (hint, offset)
        if key in seen or len(token) < 80:
            return
        seen.add(key)
        findings.append(
            Finding(
                provider_hint=hint,
                blob=sha,
                paths=paths,
                size=len(data),
                offset=offset,
                token_len=len(token),
                entropy=token_entropy(token),
                preview=token[:48] + ("…" if len(token) > 48 else ""),
            )
        )

    for hint, pat in PATTERNS:
        for m in pat.finditer(data):
            add(hint, m.group(1).decode("ascii", errors="ignore"), m.start(1))

    # reasoning_details is often a JSON-encoded string containing signature blobs
    for m in REASONING_DETAILS.finditer(data):
        raw = m.group(1).decode("utf-8", errors="ignore")
        try:
            # JSONL escapes: \" \\ \/ \n etc — use unicode_escape lightly via codecs
            unescaped = bytes(raw, "utf-8").decode("unicode_escape")
        except Exception:
            unescaped = raw.replace('\\"', '"').replace("\\\\", "\\")
        inner = unescaped.encode("utf-8", errors="ignore")
        for sm in GENERIC_SIGNATURE.finditer(inner):
            # offset approximate within outer blob
            add(
                "reasoning_details_signature",
                sm.group(1).decode("ascii", errors="ignore"),
                m.start(1) + sm.start(1),
            )

    return findings


def scan_repo(repo: Path, *, max_blob: int = 5_000_000) -> list[Finding]:
    path_index: dict[str, list[str]] = {}
    for line in git(repo, "rev-list", "--objects", "--all").stdout.splitlines():
        parts = line.split(maxsplit=1)
        if len(parts) == 2:
            path_index.setdefault(parts[0], []).append(parts[1])

    findings: list[Finding] = []
    for sha, size in list_blobs(repo):
        if size == 0 or size > max_blob:
            continue
        data = read_blob(repo, sha)
        paths = path_index.get(sha, [])[:8]
        findings.extend(scan_blob(sha, paths, data))
    return findings


def fake_b64(n: int = 200) -> str:
    return base64.b64encode(os.urandom(n)).decode().rstrip("=") + "=="


def make_demo_repo(root: Path) -> Path:
    repo = root / "demo-repo"
    if repo.exists():
        shutil.rmtree(repo)
    repo.mkdir(parents=True)
    git(repo, "init")
    git(repo, "config", "user.email", "repro@example.com")
    git(repo, "config", "user.name", "reasoning-blob-repro")

    (repo / "README.md").write_text("# demo agent logs\n", encoding="utf-8")
    git(repo, "add", "README.md")
    git(repo, "commit", "-m", "init")

    # Synthetic shapes only — not real provider ciphertext.
    anthropic = {
        "role": "assistant",
        "content": [
            {
                "type": "thinking",
                "thinking": "",
                "signature": fake_b64(180),
            },
            {
                "type": "redacted_thinking",
                "data": fake_b64(120),
            },
            {"type": "text", "text": "Visible answer only."},
        ],
    }
    openai = {
        "type": "reasoning",
        "encrypted_content": fake_b64(160),
        "summary": [{"type": "summary_text", "text": "short summary"}],
    }
    gemini = {
        "candidates": [
            {
                "content": {
                    "parts": [
                        {"text": "ok", "thoughtSignature": fake_b64(96)},
                    ]
                }
            }
        ]
    }

    (repo / "traces").mkdir()
    (repo / "traces" / "claude_session.json").write_text(
        json.dumps(anthropic, indent=2), encoding="utf-8"
    )
    (repo / "traces" / "openai_response.json").write_text(
        json.dumps(openai, indent=2), encoding="utf-8"
    )
    (repo / "traces" / "gemini_turn.json").write_text(
        json.dumps(gemini, indent=2), encoding="utf-8"
    )
    git(repo, "add", "traces")
    git(repo, "commit", "-m", "publish session logs with opaque reasoning blocks")

    # Tip delete — paper threat: still recoverable from history
    git(repo, "rm", "-r", "traces")
    git(repo, "commit", "-m", "sanitize tip (but history keeps signatures)")
    return repo


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("cmd", choices=["demo", "scan"])
    ap.add_argument("repo", nargs="?", default=".")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args()

    if args.cmd == "demo":
        tmp = Path(tempfile.mkdtemp(prefix="reasoning-blob-"))
        repo = make_demo_repo(tmp)
        print(f"[demo] fixture: {repo}", file=sys.stderr)
        print(
            "[demo] NOTE: detection only — no API decode / jailbreak",
            file=sys.stderr,
        )
    else:
        repo = Path(args.repo).resolve()
        if not (repo / ".git").exists():
            print(f"not a git repo: {repo}", file=sys.stderr)
            return 2

    findings = scan_repo(repo)
    if args.json:
        print(json.dumps([asdict(f) for f in findings], indent=2, ensure_ascii=False))
    else:
        print(f"repo: {repo}")
        print(f"encrypted reasoning-like blocks: {len(findings)}")
        by: dict[str, int] = {}
        for f in findings:
            by[f.provider_hint] = by.get(f.provider_hint, 0) + 1
        for k, v in sorted(by.items()):
            print(f"  {k}: {v}")
        for f in findings:
            path = ",".join(f.paths) if f.paths else "(blob)"
            print(
                f"- [{f.provider_hint}] {f.blob[:12]}… {path} "
                f"len={f.token_len} H={f.entropy:.2f} @ {f.offset}"
            )
            print(f"    {f.preview}")
    return 0


if __name__ == "__main__":
    # fix typo if any
    raise SystemExit(main())
