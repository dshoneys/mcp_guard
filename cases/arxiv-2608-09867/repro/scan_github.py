#!/usr/bin/env python3
"""
GitHub-scale discovery of encrypted LLM reasoning blocks
(paper-style half of arXiv:2608.09867 §4.1).

The paper did NOT mirror all of GitHub. It searched public code/datasets
for agent traces that still embed opaque AEAD envelopes, then inventoried
them. This tool mirrors that workflow:

  1) GitHub Code Search for high-signal queries
  2) Fetch matching file blobs
  3) Run local signature / encrypted_content detectors

Auth: set GH_TOKEN or GITHUB_TOKEN (fine-grained: Contents: Read +
metadata; classic: public_repo is enough for public code search).

Usage:
  set GH_TOKEN=ghp_...
  python scan_github.py --max-per-query 30 --out report.json

  # dry-run: only list search hits
  python scan_github.py --list-only

Limits (GitHub platform, not us):
  - Code Search returns at most ~1000 hits per query
  - Authenticated search ~30 req/min; code search is stricter
  - Index lag / quality filters mean this is broad, not literally exhaustive
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import asdict
from pathlib import Path

from scan_reasoning_blobs import Finding, scan_blob

API = "https://api.github.com"

# High-signal queries inspired by paper + observed ClawBench/Hermes shapes.
DEFAULT_QUERIES = [
    '"reasoning_details" signature extension:jsonl',
    '"reasoning_details" signature extension:json',
    'redacted_thinking extension:json',
    'redacted_thinking extension:jsonl',
    '"encrypted_content" reasoning extension:json',
    '"encrypted_content" extension:jsonl',
    'thoughtSignature extension:json',
    '"type":"thinking" signature extension:json',
    'agent-messages.jsonl signature',
    '"signature" "claude" path:traces extension:jsonl',
]


def token_from_gcm() -> str | None:
    """Same store git/hutao push uses (Git Credential Manager)."""
    import shutil
    import subprocess

    gcm = shutil.which("git-credential-manager") or (
        "D:/Tools/Hutao/mingw64/bin/git-credential-manager.exe"
        if os.path.isfile("D:/Tools/Hutao/mingw64/bin/git-credential-manager.exe")
        else None
    )
    if not gcm:
        return None
    try:
        proc = subprocess.run(
            [gcm, "get"],
            input="protocol=https\nhost=github.com\n\n",
            capture_output=True,
            text=True,
            timeout=15,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    password = None
    for line in proc.stdout.splitlines():
        if line.startswith("password="):
            password = line[len("password=") :].strip()
            break
    return password or None


def token() -> str | None:
    env = (
        os.environ.get("GH_TOKEN")
        or os.environ.get("GITHUB_TOKEN")
        or token_from_gcm()
    )
    if env:
        return env
    try:
        from config_loader import load_config

        t = (load_config().get("github") or {}).get("token") or ""
        return t or None
    except SystemExit:
        return None


def api_base() -> str:
    try:
        from config_loader import load_config

        return (load_config().get("github") or {}).get("api_base") or API
    except SystemExit:
        return API


def api_get(url: str, tok: str, *, accept: str = "application/vnd.github+json") -> tuple[dict | list, dict]:
    req = urllib.request.Request(
        url,
        headers={
            "Accept": accept,
            "Authorization": f"Bearer {tok}",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "mcp-guard-reasoning-scan/0.1",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            headers = {k.lower(): v for k, v in resp.headers.items()}
            body = resp.read()
            return json.loads(body.decode("utf-8")), headers
    except urllib.error.HTTPError as e:
        detail = e.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {e.code} for {url}: {detail[:500]}") from e


def search_code(tok: str, query: str, *, per_page: int = 50, max_items: int = 100) -> list[dict]:
    """Return unique {repo, path, sha, html_url} hits."""
    hits: list[dict] = []
    seen: set[tuple[str, str]] = set()
    page = 1
    while len(hits) < max_items:
        q = urllib.parse.urlencode(
            {
                "q": query,
                "per_page": min(per_page, max_items - len(hits)),
                "page": page,
            }
        )
        url = f"{api_base()}/search/code?{q}"
        data, headers = api_get(url, tok)
        items = data.get("items") or []
        if not items:
            break
        for it in items:
            repo = (it.get("repository") or {}).get("full_name") or ""
            path = it.get("path") or ""
            key = (repo, path)
            if key in seen:
                continue
            seen.add(key)
            hits.append(
                {
                    "repo": repo,
                    "path": path,
                    "sha": it.get("sha"),
                    "html_url": it.get("html_url"),
                    "query": query,
                }
            )
            if len(hits) >= max_items:
                break
        # secondary rate limit courtesy
        remaining = headers.get("x-ratelimit-remaining")
        if remaining is not None and remaining.isdigit() and int(remaining) < 2:
            reset = int(headers.get("x-ratelimit-reset") or "0")
            wait = max(0, reset - int(time.time())) + 1
            print(f"[rate] sleeping {wait}s", file=sys.stderr)
            time.sleep(wait)
        else:
            time.sleep(2.5)  # code search is sensitive
        if len(items) < per_page:
            break
        page += 1
        if page > 10:  # GitHub hard cap ~1000
            break
    return hits


def fetch_raw(tok: str, repo: str, path: str, *, max_bytes: int = 2_000_000) -> bytes | None:
    # Contents API returns base64 for files; use raw media type.
    url = f"{api_base()}/repos/{repo}/contents/{urllib.parse.quote(path)}"
    req = urllib.request.Request(
        url,
        headers={
            "Accept": "application/vnd.github.raw",
            "Authorization": f"Bearer {tok}",
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "mcp-guard-reasoning-scan/0.1",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = resp.read(max_bytes + 1)
            if len(data) > max_bytes:
                return data[:max_bytes]
            return data
    except urllib.error.HTTPError as e:
        print(f"[skip] {repo}/{path}: HTTP {e.code}", file=sys.stderr)
        return None


def scan_hit(tok: str, hit: dict) -> list[dict]:
    blob = fetch_raw(tok, hit["repo"], hit["path"])
    if blob is None:
        return []
    findings = scan_blob(hit["sha"] or "remote", [f"{hit['repo']}:{hit['path']}"], blob)
    out = []
    for f in findings:
        row = asdict(f)
        row["repo"] = hit["repo"]
        row["path"] = hit["path"]
        row["html_url"] = hit.get("html_url")
        row["query"] = hit.get("query")
        out.append(row)
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--query", action="append", help="Extra/override search query (repeatable)")
    ap.add_argument("--max-per-query", type=int, default=40)
    ap.add_argument("--list-only", action="store_true", help="Only list search hits")
    ap.add_argument("--out", type=Path, default=Path("github_reasoning_report.json"))
    ap.add_argument("--cache-dir", type=Path, default=Path("fixtures/github-hits"))
    args = ap.parse_args()

    tok = token()
    if not tok:
        print(
            "缺少 GitHub token。\n"
            "优先级: GH_TOKEN / GITHUB_TOKEN → Git Credential Manager（与 hutao push 同源）→ gh auth。\n"
            "  gh auth login -h github.com -p https -w\n"
            "  或: $env:GH_TOKEN = \"ghp_...\"",
            file=sys.stderr,
        )
        return 2
    src = (
        "env"
        if (os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN"))
        else "git-credential-manager"
    )
    print(f"[auth] using token from {src}", file=sys.stderr)

    queries = args.query or DEFAULT_QUERIES
    all_hits: list[dict] = []
    seen: set[tuple[str, str]] = set()
    for q in queries:
        print(f"[search] {q}", file=sys.stderr)
        try:
            hits = search_code(tok, q, max_items=args.max_per_query)
        except Exception as e:
            print(f"[error] query failed: {e}", file=sys.stderr)
            continue
        print(f"  -> {len(hits)} hits", file=sys.stderr)
        for h in hits:
            key = (h["repo"], h["path"])
            if key in seen:
                continue
            seen.add(key)
            all_hits.append(h)

    print(f"[total unique files] {len(all_hits)}", file=sys.stderr)
    if args.list_only:
        print(json.dumps(all_hits, indent=2, ensure_ascii=False))
        return 0

    args.cache_dir.mkdir(parents=True, exist_ok=True)
    findings: list[dict] = []
    files_with_hits = 0
    for i, hit in enumerate(all_hits, 1):
        print(f"[fetch {i}/{len(all_hits)}] {hit['repo']}/{hit['path']}", file=sys.stderr)
        rows = scan_hit(tok, hit)
        if rows:
            files_with_hits += 1
            findings.extend(rows)
            # optional cache
            safe = hit["repo"].replace("/", "__") + "__" + hit["path"].replace("/", "__")
            (args.cache_dir / f"{safe}.meta.json").write_text(
                json.dumps({"hit": hit, "n_findings": len(rows)}, indent=2),
                encoding="utf-8",
            )
        time.sleep(0.4)

    report = {
        "paper": "arXiv:2608.09867",
        "mode": "github_code_search",
        "queries": queries,
        "files_searched": len(all_hits),
        "files_with_blocks": files_with_hits,
        "blocks_found": len(findings),
        "findings": findings,
        "note": (
            "GitHub Code Search is indexed + capped (~1000/query); "
            "this approximates the paper's public scrape, not a literal "
            "byte-for-byte mirror of every public repo."
        ),
    }
    args.out.write_text(json.dumps(report, indent=2, ensure_ascii=False), encoding="utf-8")
    print(
        f"done: files={len(all_hits)} with_blocks={files_with_hits} "
        f"signatures={len(findings)} -> {args.out}",
        file=sys.stderr,
    )
    # summary by provider hint
    by: dict[str, int] = {}
    for f in findings:
        by[f["provider_hint"]] = by.get(f["provider_hint"], 0) + 1
    for k, v in sorted(by.items(), key=lambda kv: -kv[1]):
        print(f"  {k}: {v}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
