# MCP Guard — Agent 角色与权限

## 原则

1. **ADL 先于代码**（见 `doc/structurizr/ADL-RULES.md`）。
2. **最高权限者 = 最高开发者（Lead）**：可直接在 `master` 上提交/推送；负责架构与最终合并质量。
3. **其它角色**：只能 **认领 Issue → 建分支 → 提 PR**；**禁止**直推 `master`。

## 角色

| 角色 ID | 名称 | 权限 | 职责 |
|---------|------|------|------|
| `lead` | Lead / Architect-Developer | **直推 `master`**；开/关 Issue；改 ADL；合 PR；紧急热修；可兼设计 | 维护 `graph.json` / `requirements.json` / `workspace.dsl`；拆 Issue；保证 O(1) 与 \(R_{\mathrm{manual}}\)；可写任意模块 |
| `designer` | UX/UI Designer | 认领设计 Issue；分支；PR；只改 `doc/ux/**`、`doc/ui/**`、`ui/preview/**`、`ui/*.toml`、`ui/tokens.css` | UX 文档 + Code-as-Design 预览；不写业务 `src/**` |
| `module` | Module Agent | 认领 Issue；分支开发；提 PR | 只改 Issue 指定的 `component`；有 UI 时须 `ux_status: accepted`；不擅自改依赖边 |
| `integrator` | Integrator | 认领 Issue；分支；PR | 只改 compose/runtime 注册与 CI 接线；不写插件业务规则 |
| `reviewer` | Reviewer | 评论 PR；跑检查；**不合入、不推 master** | 跑 `adl_check.py`；指出出度/差集/越权/缺 UX|UI；不写功能代码 |

同一自然人可兼任；**机器 Agent 必须在 Issue/PR 上声明当前角色**。

### Cursor 默认身份

本机打开 MCP Guard（含在 kuroneko 下编辑 `experiments/mcp_guard/**`）时的 **Cursor Agent 固定为 `lead`**（见仓库根 `AGENTS.md` 与 `.cursor/rules`）。  
其它 Cursor 窗口 / 云端 Agent 若扮演 module，须在会话开头声明 `role: module` 且不得推 `master`。

## 权限矩阵

| 动作 | lead | designer | module | integrator | reviewer |
|------|:----:|:--------:|:------:|:----------:|:--------:|
| 推送 `master` | ✅ | ❌ | ❌ | ❌ | ❌ |
| 修改 `doc/structurizr/model/*`、`workspace.dsl` | ✅ | ❌ | ❌* | ❌* | ❌ |
| 修改 `doc/ui/**`、`doc/ux/**`、`ui/preview/**`、`ui/*.toml` | ✅ | ✅（认领范围） | ❌ | ❌ | ❌ |
| 修改 `scripts/adl_check.py` | ✅ | ❌ | ❌ | ⚠️ CI 相关可 PR | ❌ |
| 修改指定 plugin 源码 | ✅ | ❌ | ✅（仅认领范围） | ❌ | ❌ |
| 修改 `src/main.rs` / runtime 组装 | ✅ | ❌ | ❌ | ✅（仅认领范围） | ❌ |
| 开 Issue / 指派 | ✅ | 可建议 | 可建议 | 可建议 | 可建议 |
| 合 PR 进 `master` | ✅ | ❌ | ❌ | ❌ | ❌ |

\* Module/Integrator 若实现 **必须** 改 ADL：先开 Issue 标 `needs-adl`，由 **lead** 改模型并挂上 Issue 后再动手；或由 lead 在同迭代先推 ADL 到 `master`。  
有 UI 的实现：先 **`ux_status: accepted`**；视觉阶段再 **`ui_status: accepted`**（见 [`UI-DESIGN.md`](./UI-DESIGN.md)）。

## 分支命名

```text
agent/<role>/<issue-number>-<short-slug>

示例:
  agent/module/12-scan-http-headers
  agent/designer/20-tray-ui
  agent/integrator/15-runtime-dyn-scanner
```

`lead` 热修可用：`lead/hotfix/<slug>`（仍建议事后补 Issue）。

## Issue 认领

1. Issue 必须含：`req-id`、`component`、`role`、`test_kind` 预期。
2. 非 lead 在评论写：`claim: module` / `claim: designer` / `claim: integrator` 后开始分支。
3. 同时只认领 **一个** 进行中 Issue（除非 lead 书面放开）。
4. PR 必须链接 Issue；CI / 本地 `python scripts/adl_check.py` 必须绿。
5. 有 UI：先 `[UX]` Issue → 实现 → `[UI]` Issue；实现前须 `ux_status: accepted`。

## Lead 直推 master 的约定

允许，但建议：

- 仍跑 `adl_check.py`
- 提交信息带 `req-id`（若有）
- 结构性变更同步 `workspace.dsl` + `graph.json` + `requirements.json`
- 避免长时间本地堆积；直推不等于跳过 ADL
