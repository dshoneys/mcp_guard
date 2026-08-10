# 2. Agent 角色：Lead 直推 master，其余认领 Issue 分支开发

## Status

Accepted

## Context

多 Agent / 多人协作需要权限差，同时 Lead 也是最高开发者，不能被流程堵死。

## Decision

- 角色：`lead` | `module` | `integrator` | `reviewer`（见 `doc/agents/ROLES.md`）。
- **仅 lead** 可直接推送 `master` / 合 PR。
- 其它角色只能认领 Issue、在 `agent/<role>/…` 分支开发并开 PR。
- ADL 文件由 lead 拥有（CODEOWNERS）；CI 跑 `adl_check.py`。

## Consequences

- GitHub 建议：`master` 限制推送，但 **Allow administrators / lead 绕过**（或仅 lead 有写权限）。
- 机器 Agent 会话必须先声明角色（`AGENTS.md`）。
