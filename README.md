# MCP Guard

<p align="center">
  <img src="ui/brand/logo.png" alt="MCP Guard logo" width="160" />
</p>

<p align="center">
  <strong>中文</strong> · <a href="README.en.md">English</a>
</p>

本机代理：对机器上的 **MCP / Agent 工具调用面** 做 **扫描、监视、审计**（后续再做硬拦截）。

仓库：https://github.com/shinjiyu/mcp_guard

当前版本：`0.1.0-beta.1`（预发布）

## 它做什么

MCP Guard **不**跑在浏览器里。它作为主机侧 Agent，盯住共同瓶颈：**谁能访问本机 MCP 端口**。

```text
  恶意网页 / 脚本 / Electron / curl
              │
              ▼
        127.0.0.1:50551  （示例：WorkBuddy Ardot MCP）
              │
    ┌─────────┴──────────────────────────────────┐
    │              MCP Guard（本机）                │
    │  1. scan  — 枚举端口；探测未保护的 MCP        │
    │  2. watch — 谁在监听 / 谁在连接？             │
    │  3. audit — JSONL 审计轨迹                   │
    │  4. gate  — （后续）拦截未知客户端             │
    └─────────┬──────────────────────────────────┘
              ▼
         MCP 服务进程
```

### 架构权威（ADL）

多 Agent / O(1) 规则见 [`doc/structurizr/`](doc/structurizr/README.md)：

- 插件出度 O(1)（`model/graph.json`）
- \(R_{\mathrm{manual}}=R\setminus R_U\)（`model/requirements.json`）→ 需人测项
- 门禁：`python scripts/adl_check.py`
- 角色：[`AGENTS.md`](AGENTS.md)

### 流水线

1. **发现 / 探测（`scan`）**  
   枚举 loopback 监听端口 → HTTP 指纹 → POST MCP `tools/list`。  
   **只有探测到未保护的 MCP** 才记风险：  
   `mcp_jsonrpc_surface`（警告）、`mcp_tools_exposed`（进一步风险）；  
   确认 MCP 后才可能附带 `cors_star` / 鉴权提示 / WorkBuddy 端口标记。  
   普通本机 HTTP **不计** warning。

2. **归因（`watch`）**  
   读系统 TCP 表，映射套接字 → PID → 进程名/路径；对照 `gate.allow_process_names`。  
   未知客户端 → **ACTIVITY ALERT** + 审计 `activity_alert`。

3. **记录（`serve`）**  
   循环：scan + watch → 追加 `mcp-guard-audit.jsonl`。

4. **硬闸门（尚未实现）**  
   Windows WFP / macOS pf / Linux nftables（或用户态代理），需归因足够可靠后再做。

| 告警 | 何时 |
|------|------|
| `exposure_alert` | 发现未保护的 MCP 风险面 |
| `activity_alert` | 非白名单进程正在连接该端口 |

浏览器扩展只能作演示——挡不住任意能开 loopback 套接字的客户端。

## 能力一览

| 能力 | 状态 |
|------|------|
| `mcp-guard scan` | ✅ |
| `mcp-guard watch` | ✅ 软归因 |
| `mcp-guard serve` | ✅ scan + watch + 审计 |
| `mcp-guard tray` | ✅ 托盘 + 主界面 + 后台 Agent（**默认中文**；`--locale en`） |
| `mcp-guard dashboard` | ✅ 仅主窗口（无托盘；日常用 `tray`） |
| `mcp-guard status` | ✅ 菜单模型 + 审计快照 JSON |
| `mcp-guard vault` / `vault-mcp` | ✅ NoContext 密钥保险箱 |
| 硬端口/进程拦截 | ⏳ |
| 路径 / 工具策略 | ⏳ |

### 日常使用

```bash
cargo build --release
./target/release/mcp-guard tray
```

- 启动后自动扫描一次  
- 最小化 / 关闭 → **藏到托盘**；托盘左键或「打开主界面」恢复  
- 真正退出：托盘菜单「退出」

可选配置：复制 `mcp-guard.toml.example` → `mcp-guard.toml`。

### 密钥保险箱（NoContext）

Agent **不应**通过 MCP 工具结果拿到明文密钥（会进入对话上下文）。见 [`doc/structurizr/VAULT-NOCONTEXT.md`](doc/structurizr/VAULT-NOCONTEXT.md)。

```bash
mcp-guard vault put openai --value "sk-..."
mcp-guard vault list
mcp-guard vault issue-ref openai
# Cursor / Claude Desktop 的 mcp.json：
# { "command": "mcp-guard", "args": ["vault-mcp"] }
```

工具：`vault_list`、`vault_issue_ref`、`vault_ref_info`、`vault_run_with_secret` — **没有** `vault_get`。

## 构建与调试

```bash
cargo build --release
./target/release/mcp-guard scan
./target/release/mcp-guard watch
./target/release/mcp-guard serve --once
```

```bash
RUST_LOG=debug mcp-guard scan --ports 50551,52412
```

## 许可证

MIT
