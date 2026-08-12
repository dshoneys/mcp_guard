# Gitee

## 项目简介（仓库「项目介绍」栏，约 200 字）

厂商把模型的思维链（CoT）做成「加密签名块」让客户端回传——看起来安全。论文 [arXiv:2608.09867](https://arxiv.org/abs/2608.09867) 指出：这些不透明块一旦写进 **Git**，同厂商较弱模型仍可能通过 API **回放展开**明文推理。

**MCP Guard** 是本机 Agent 安全哨兵：扫描未保护的 MCP 端口、监视谁在连 loopback、审计告警，并新增 **`git-scan`**——在 commit 前拦住 reasoning / thinking.signature 等密文入库。

国内交流与 Issue 请优先提在本仓；国际镜像见 GitHub。复现与对照页见 `cases/arxiv-2608-09867/`。

---

## 发布动态（短）

标题：论文说的「加密思维链」进 Git 能被展开——我们复现了，并做了防入库

正文：

最近 arXiv:2608.09867 讨论一类很隐蔽的泄露：Agent 轨迹里的 `thinking.signature` / `encrypted_content` 看起来是乱码，但同厂商 API 可以 past_turn 回放成明文 CoT。我们在 MCP Guard 里放了可跑的复现 case（脱敏 config + 脚本），并加了 `mcp-guard git-scan`，防止自己把密文提交进仓库。

- Case：`cases/arxiv-2608-09867/`  
- 防护：`mcp-guard git-scan --staged .`  
- 本仓 Issue 欢迎吐槽与贡献  

仓库：https://gitee.com/shinjiyu/mcp_guard
