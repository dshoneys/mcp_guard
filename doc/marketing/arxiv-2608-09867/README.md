# 宣传文稿索引（arXiv:2608.09867 × MCP Guard）

以论文披露的 **CoT / reasoning 密文可被同厂商弱模型回放展开** 为引子，推广：

1. 复现 case：`cases/arxiv-2608-09867/`
2. 产品防护：`mcp-guard git-scan` + MCP 本机哨兵

| 文件 | 平台 |
|------|------|
| [`gitee.md`](./gitee.md) | Gitee 项目介绍 / 动态 |
| [`github.md`](./github.md) | GitHub README 短引 / Discussion / Release |
| [`wechat.md`](./wechat.md) | 公众号长文 + 朋友圈短句 |
| [`zhihu.md`](./zhihu.md) | 知乎回答 / 想法 |
| [`v2ex-oschina.md`](./v2ex-oschina.md) | V2EX / 开源中国 |
| [`x-en.md`](./x-en.md) | X / LinkedIn（英文） |
| [`one-liners.md`](./one-liners.md) | 标题备选 / 短标语 |

**链接（发文时替换一致）：**

- GitHub：https://github.com/shinjiyu/mcp_guard  
- Gitee：https://gitee.com/shinjiyu/mcp_guard  
- 论文：https://arxiv.org/abs/2608.09867  
- Case：仓库内 `cases/arxiv-2608-09867/`  
- 对照页：`cases/arxiv-2608-09867/assets/compare_cipher_plain.html`

**表述边界（避免夸大）：**

- 我们复现的是「签名块入库 + 同厂商 past_turn 回放」，不是本地硬破解 AEAD。  
- Fig.33 current-turn 在部分网关上已失效；past_turn + Sonnet/Opus 仍可能有效。  
- `git-scan` 防的是**把密文提交进 git**；不替代厂商侧密钥轮换。
