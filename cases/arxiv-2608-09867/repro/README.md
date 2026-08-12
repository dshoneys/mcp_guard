# Lab scripts for arXiv:2608.09867 (optional)

**Playbook (中文步骤):** [`../REPRO-PLAN.md`](../REPRO-PLAN.md)

These Python helpers are for **offline detection** and **authorized API replay**.
They are not required to use `mcp-guard git-scan`.

| Script | Role | Phase |
|--------|------|-------|
| `scan_reasoning_blobs.py` | Local detectors + demo fixture | A1–A2 |
| `scan_github.py` | GitHub Code Search sampling (needs token) | A3 |
| `decode_past_turn.py` | PocketCity past_turn decode (needs LOCALMODULE_*) | C |

Do not commit live signatures, API keys, or full foreign decrypt dumps into this folder.
