---
name: Agent task
about: Claimable work item for module/integrator (lead 拆单)
title: "[REQ-xxx] "
labels: agent-task
---

## req-id
`REQ-`

## component
`scan` | `watch` | `audit` | `gate` | `runtime` | `config` | `cli` | `contracts`

## role（谁可认领）
- [ ] module
- [ ] integrator
- [ ] lead-only（含 ADL）

## test_kind
`unit` | `contract` | `manual`

## Acceptance
<!-- 与 requirements.json 对齐 -->

## Paths allowed
<!-- 例如 src/scan.rs, tests/... -->

## Claim
非 lead 评论：`claim: module` 或 `claim: integrator` 后开分支  
`agent/<role>/<issue-number>-<slug>`
