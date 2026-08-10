## Summary
<!-- 做了什么、对应哪个 req-id -->

## Role
- [ ] lead
- [ ] designer
- [ ] module
- [ ] integrator
- [ ] reviewer（仅评论则勿开功能 PR）

## Issue
Fixes #

## ADL
- [ ] 未改 `graph.json` / `requirements.json` / `workspace.dsl`
- [ ] 已由 lead 在 master 更新 ADL（链接 commit/PR）
- [ ] 本 PR 含 ADL（**仅 lead**）

## UI design
- [ ] 无 UI（`ui:false`）
- [ ] UX PR（仅 `doc/ux/**`）
- [ ] 实现 PR：`ux_status: accepted`；壳不写死视觉
- [ ] UI PR（`doc/ui/**` 和/或 `ui/` config）；`ui_impl` = config|code|hybrid

## Checks
- [ ] `python scripts/adl_check.py` 通过
- [ ] 未引入 plugin→plugin 依赖
- [ ] 未修改认领范围外的路径

## R_manual
- [ ] 无新人测项
- [ ] 有：已在 Issue 列出人测步骤（不靠单测假装覆盖）
