# Offense / defense cases

Curated **攻防 case packs** for MCP Guard: each folder is one threat narrative with
repro notes, defense wiring, and fixtures.

| Case | Threat | Defense in product |
|------|--------|--------------------|
| [`arxiv-2608-09867`](./arxiv-2608-09867/) | Stealing Reasoning Traces — opaque CoT AEAD blobs mined from git | `mcp-guard git-scan` + pre-commit hooks |

Lab / exploratory scripts may also live under `experiments/` (not required for CI).
