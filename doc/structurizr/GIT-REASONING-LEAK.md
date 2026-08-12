# Git reasoning-leak scan (REQ-GIT-REASONING-LEAK)

Defend against [arXiv:2608.09867](https://arxiv.org/abs/2608.09867): opaque provider CoT / reasoning AEAD blobs (`thinking.signature`, `encrypted_content`, `thoughtSignature`) leaking into **local git** history, where they can later be mined from public clones.

## Pipeline

1. Resolve git exe (`hutao` if on PATH, else `git`).
2. List targets:
   - default: `git ls-files` (tracked working tree)
   - `--staged`: `git diff --cached --name-only --diff-filter=ACMR`
3. Read each file (cap `git_scan.max_file_bytes`); skip obvious binaries (`\\0` in first 2KiB) unless they contain reasoning keywords.
4. Match detectors (same family as case pack / paper):
   - Anthropic `type:thinking` + `signature`
   - OpenAI-style `encrypted_content`
   - Gemini `thoughtSignature`
   - Generic long `"signature": "…"` fields (≥80 b64-ish chars)
5. Emit `GitScanReport` JSON; with `--fail` (default) exit **1** if any finding.

## Paths

- `src/git_scan.rs` — detectors + tree walk
- `src/contracts.rs` — `GitScanReport` / `GitScanFinding`
- `src/config.rs` — `[git_scan]`
- CLI: `mcp-guard git-scan [PATH]`
- Case pack: `cases/arxiv-2608-09867/`
- Hook samples: `cases/arxiv-2608-09867/defense/`

## Non-goals

- Does **not** implement cross-model decrypt / jailbreak helpers in the binary.
- History-wide `rev-list` scan is optional future work (`--history` not in v1).
