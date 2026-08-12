# X / LinkedIn (English)

## X (thread-ready)

1/ Providers wrap chain-of-thought as “encrypted” signatures. [arXiv:2608.09867](https://arxiv.org/abs/2608.09867) shows: once those blobs land in git, same-vendor APIs can still replay-decode them.

2/ We packaged a runnable offense/defense case + a product control: `mcp-guard git-scan` blocks `thinking.signature` / `encrypted_content` / `thoughtSignature` from being committed.

3/ MCP Guard is a local MCP sentinel (scan unprotected loopback MCP, watch peers, audit).  
GitHub: https://github.com/shinjiyu/mcp_guard  
Gitee (CN): https://gitee.com/shinjiyu/mcp_guard

---

## LinkedIn (short post)

Security note for teams shipping agent traces:

Opaque CoT blobs in git are not a vault. Per arXiv:2608.09867, same-provider replay can recover reasoning text. We open-sourced a reproduction case and added `mcp-guard git-scan` so those fields fail the commit gate—alongside MCP Guard’s existing local MCP exposure scanning.

https://github.com/shinjiyu/mcp_guard
