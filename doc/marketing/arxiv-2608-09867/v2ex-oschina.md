# V2EX / 开源中国

## V2EX（分享创造）

标题：复现了「Git 里的加密 CoT 可被回放展开」，并给 MCP 本机哨兵加了 git-scan

正文：

论文 arXiv:2608.09867：厂商把思维链做成不透明 AEAD 块（thinking.signature 等），一旦进 git，同厂商 API 仍可能 past_turn 转录明文。

我们在 MCP Guard 里放了攻防 case（可跑脚本 + 脱敏 config + 对照 HTML），产品侧加了：

```
mcp-guard git-scan --staged .
```

顺便：这个项目本来就是扫本机未保护 MCP、监视 loopback 连接、审计 JSONL 的。

- 国内：https://gitee.com/shinjiyu/mcp_guard  
- 国际：https://github.com/shinjiyu/mcp_guard  

求拍：你们的 agent 轨迹有没有整段 signature 入库的习惯？

---

## 开源中国 / 软件更新资讯

**一句话：** MCP Guard 新增 git-scan，防御 arXiv:2608.09867 所述「推理签名块」误入版本库，并附带可复现 case。

**详情：**  
MCP Guard 是面向本机 MCP / Agent 工具面的扫描与审计工具。本次更新引入对 Anthropic / OpenAI / Gemini 等 opaque reasoning 字段的检测，支持 tracked 与 staged 扫描，可挂 pre-commit。配套 `cases/arxiv-2608-09867` 提供论文相关复现脚本与密文—明文对照页。国内托管：https://gitee.com/shinjiyu/mcp_guard
