# 微信

## 公众号长文（可直接改标题后发）

**标题备选：**  
1. 加密的「思维链」写进 Git，别人还能展开？我们复现了一篇 arXiv，并做了拦截  
2. Agent 轨迹里的 signature 不是乱码那么简单  
3. 从一篇「偷推理痕迹」论文，到本机 MCP 哨兵的一次补丁  

**正文：**

很多团队以为：厂商把 CoT（思维链）做成加密块，客户端只存 signature，就不会泄露「模型怎么想的」。

论文 *Stealing Reasoning Traces from Proprietary LLM APIs*（arXiv:2608.09867）指出另一条路——**不需要硬破解 AEAD**。只要这些不透明块出现在公开或可访问的 Git 历史里，攻击者可以用**同厂商**的 API，按特定对话模板（past_turn + 空 thinking + 真 signature）把明文推理「回放」出来。

我们做了两件事：

**一、复现库（攻防 case）**  
在 MCP Guard 仓库的 `cases/arxiv-2608-09867/` 里放了可跑脚本：脱敏配置 `config.example.toml`、控制实验（证明不是简单复述 prompt）、以及密文↔明文对照页。国内同学可从 Gitee 拉仓。

**二、产品防护**  
新增命令：

```bash
mcp-guard git-scan .
mcp-guard git-scan --staged .   # 适合挂 pre-commit
```

在 commit 前扫描 `thinking.signature`、`encrypted_content`、`thoughtSignature` 等字段，避免自己把「加密思维链」送进仓库。

MCP Guard 本身还是本机侧的 MCP 哨兵：扫未保护的 loopback MCP、看谁在连、写审计日志；密钥侧有 NoContext vault，避免密钥进对话上下文。

链接：  
- Gitee：https://gitee.com/shinjiyu/mcp_guard  
- GitHub：https://github.com/shinjiyu/mcp_guard  
- 论文：https://arxiv.org/abs/2608.09867  

欢迎 Issue 反馈你们线上是否扫到过类似块。

---

## 朋友圈 / 社群短句（三选一）

1. 论文说：进了 Git 的「加密 CoT」还能被同厂商 API 展开。我们复现了，并给 MCP Guard 加了 git-scan 防入库。Gitee：shinjiyu/mcp_guard  

2. Agent 轨迹里的 thinking.signature ≠ 真的不能读。复现 case + 本机拦截：mcp-guard git-scan  

3. 不是破密码，是回放。CoT 密文别往 git 里扔——MCP Guard 帮你挡一刀。
