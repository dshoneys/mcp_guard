# GitHub

## README teaser（可插在 README 顶部「新能力」）

> **New (defense case):** Providers wrap chain-of-thought as opaque AEAD blobs. [arXiv:2608.09867](https://arxiv.org/abs/2608.09867) shows they can still be replay-decoded via same-vendor APIs once they land in git. We ship a runnable lab under [`cases/arxiv-2608-09867`](../../cases/arxiv-2608-09867) and **`mcp-guard git-scan`** to block those blobs from being committed.

中文交流：https://gitee.com/shinjiyu/mcp_guard

---

## Discussion / Announcement

**Title:** Opaque CoT signatures in git aren’t as opaque as they look (arXiv:2608.09867) — repro + git-scan defense

We reproduced the “reasoning trace in git → same-provider past_turn transcription” path described in [arXiv:2608.09867](https://arxiv.org/abs/2608.09867), and packaged it as an offense/defense case:

- Lab scripts + desensitized `config.example.toml`: `cases/arxiv-2608-09867/repro/`
- Ciphertext ↔ plaintext compare page in `assets/`
- Product defense: `mcp-guard git-scan` (tracked / `--staged`) so agent traces don’t quietly enter history

MCP Guard also continues to scan unprotected local MCP surfaces, watch loopback peers, and audit to JSONL.

China users / Issues: https://gitee.com/shinjiyu/mcp_guard  
Repo: https://github.com/shinjiyu/mcp_guard

---

## Release notes 片段

```
### Security case: reasoning-trace leak (arXiv:2608.09867)
- Add cases/arxiv-2608-09867 (repro scripts + compare HTML)
- Add mcp-guard git-scan to detect Anthropic/OpenAI/Gemini opaque reasoning blobs in git trees
- Docs: doc/structurizr/GIT-REASONING-LEAK.md
```
