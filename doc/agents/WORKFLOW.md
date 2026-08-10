# 多 Agent 工作流

```text
Lead: ADL + requirements
        ↓
① designer UX  → doc/ux/**  → ux_status accepted
        ↓
② module/integrator 实现行为（stub shell 可）
        ↓
③ designer UI → doc/ui/** + ui/ config|code → ui_status accepted
        ↓
Reviewer → Lead merge
```

细则：[`UI-DESIGN.md`](./UI-DESIGN.md)（含 **UI as Config/Code**）。

## Module / Integrator 步骤

1. 认领 Issue（评论 `claim: <role>`）
2. `git fetch && git checkout -b agent/<role>/<n>-<slug> origin/master`
3. 若缺 ADL → `blocked: needs-adl`
4. 若 `ui:true` 且 `ux_status` 非 accepted → `blocked: needs-ux`
5. 实现行为 + 单测；UI 壳只接契约，皮肤留给 ③
6. `python scripts/adl_check.py`
7. PR → `master`；不自合并
8. \(R_{\mathrm{manual}}\) 勾人测

## Designer 步骤

**UX Issue `[UX]`：** 只写 `doc/ux/<req-id>/` → lead 置 `ux_status: accepted`  
**UI Issue `[UI]`：** 在 UX accepted 且行为可演示后，写 `doc/ui/<req-id>/` + 必要时 `ui/` 配置 → `ui_status: accepted`  
不改 `src/**` 业务插件。

## Lead 步骤

1. 维护 requirements（`ux_*` / `ui_*` / `ui_impl`）与 graph（含 `ui_shell`）
2. 先拆 UX Issue，再拆实现，再拆 UI Issue
3. `adl_check.py`；合 PR 或直推 master
4. 维护 \(R_{\mathrm{manual}}\)

## 越权处理

- PR 改了未授权路径 → reviewer/lead 要求拆分或关闭
- PR 增加 plugin→plugin 边 → **必须拒**；先 ADL
- 非 lead 推了 master → 视为事故：revert + 改用分支
