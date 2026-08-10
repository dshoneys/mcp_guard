---
name: UI design (Code as Design)
about: Step ③ — HTML/CSS preview required; no Figma
title: "[UI][REQ-xxx] "
labels: ui-design
---

## req-id
`REQ-`

## Prerequisites
- [ ] `ux_status: accepted`
- [ ] 行为可演示（stub 亦可）

## role
- [x] designer

## Deliverables（强制可预览）
- [ ] `ui/preview/<REQ-ID>/index.html`（浏览器可看）
- [ ] `doc/ui/<REQ-ID>/brief.md`
- [ ] `doc/ui/<REQ-ID>/mapping.md`
- [ ] `doc/ui/<REQ-ID>/acceptance.md`
- [ ] token 与 `ui/default.toml` / `ui/tokens.css` 对齐

## Preview
```bash
python scripts/ui_preview.py
```

## ui_impl
`config` | `code` | `hybrid`

## Out of scope
Figma；不改 UX 语义；不改业务插件。

## Claim
`claim: designer` → `agent/designer/<issue>-ui-<slug>`
