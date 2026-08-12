# 知乎

## 想法（短）

厂商给的 thinking.signature 看起来像乱码，论文 arXiv:2608.09867 证明：进 Git 后仍可能被同厂商弱模型 past_turn 展开成明文 CoT。我们做了可跑复现 + `mcp-guard git-scan` 防入库。仓库：https://gitee.com/shinjiyu/mcp_guard

---

## 回答体（问题示例：「Agent 日志 / 轨迹提交到 GitHub 有什么风险？」）

除了明文 prompt、密钥、客户数据，还有一类更隐蔽的：**推理密文块**。

很多 API 会返回 `thinking` + `signature`（或 OpenAI 的 `encrypted_content`、Gemini 的 `thoughtSignature`）。团队常把整段 JSON 轨迹提交进仓库「方便复盘」。arXiv:2608.09867 指出，这些块并不是「只有厂商能解」那么简单——同厂商兼容 API 上，用特定历史拼装可以把 CoT 转录出来。

实践建议：

1. 轨迹脱敏：不要提交完整 signature / encrypted_content。  
2. 用扫描器挡 commit：我们在开源项目 **MCP Guard** 里加了 `git-scan`，并附带复现 case（`cases/arxiv-2608-09867`），方便安全同学自测。  
3. 本机 MCP 端口也要管：未鉴权的 loopback MCP 是另一条常见暴露面，这是 MCP Guard 的主战场。

Gitee（国内 Issue）：https://gitee.com/shinjiyu/mcp_guard  
GitHub：https://github.com/shinjiyu/mcp_guard  
论文：https://arxiv.org/abs/2608.09867
