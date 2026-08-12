# 复现文案 = 直接跑脚本（密钥只放 config.toml）

**不必注册 GitCode。** 本 case 挖密文用 **GitHub Code Search**（本机已有 GitHub 登录/token 即可）。  
GitCode 只在你要「加速 clone 某个已知仓」时才有用，公开浏览镜像通常也不强依赖帐号。

---

## 1. 配置（脱敏模板已入库）

```bash
cd cases/arxiv-2608-09867/repro
cp config.example.toml config.toml
# 编辑 config.toml：填 [llm] base_url / api_key（example 里是假地址）
# 可选 [github] token；也可用环境变量 GH_TOKEN / LLM_API_KEY
```

`config.toml` **不要提交**（见 `.gitignore`）。

---

## 2. 脚本入口

| 命令 | 作用 |
|------|------|
| `python run_repro.py demo` | 本地检测器自测（无 API） |
| `python run_repro.py critical` | 铸 signature → past_turn 展开（控制实验，需 LLM） |
| `python run_repro.py github` | GitHub 抽样扫密文 |
| `python run_repro.py decode` | 对扫到的密文做 past_turn 解码 |
| `python run_repro.py all` | demo → critical（有 token 再 github+decode） |

单脚本：

- `repro_critical.py` — marker 不在明文历史时仍泄漏 ⇒ 真解密  
- `scan_reasoning_blobs.py` / `scan_github.py` / `decode_past_turn.py`

产物默认写 `out/`（已 gitignore）。

---

## 3. 防御对照

```bash
mcp-guard git-scan .
mcp-guard git-scan --staged .
```

对照页：[`assets/compare_cipher_plain.html`](./assets/compare_cipher_plain.html)

细节矩阵见 [`CASE.md`](./CASE.md)。
