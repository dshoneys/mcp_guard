# arXiv:2608.09867 — 使用说明

## 防御（产品能力）

```bash
# 扫描当前仓库已跟踪文件
mcp-guard git-scan .

# 只扫暂存区（适合 pre-commit）
mcp-guard git-scan --staged .

# 有命中仍打印 JSON、但不以非零退出
mcp-guard git-scan . --fail false
```

Hook 示例：[`defense/`](./defense/)

配置（可选 `mcp-guard.toml`）：

```toml
[git_scan]
max_file_bytes = 5000000
extensions = [".json", ".jsonl", ".ndjson", ".txt", ".md", ".log", ".yml", ".yaml"]
```

## 对照页

打开 [`assets/compare_cipher_plain.html`](./assets/compare_cipher_plain.html) 查看 GitHub 外源密文与展开 CoT 对照（实验室结果快照）。

## 进攻复现（实验室，需自备 API）

`repro/` 内脚本依赖 PocketCity / Anthropic 兼容 `messages` API。**不要**把真实 signature / 密钥提交进本仓。

```bash
# 最小检测器自测（无 API）
python repro/scan_reasoning_blobs.py demo

# 完整解码路径见 CASE.md；实验草稿仍可能在 experiments/git-cipher-scan/
```

## 扫描范围说明

公开 GitHub 检索是 **Code Search 抽样**（约 10 条 query × 每条 ≤40 命中），不是全站镜像。本地防御扫描覆盖 **本仓库 tracked / staged** 文件。
