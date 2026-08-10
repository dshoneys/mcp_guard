# UX → 实现 → UI + Code-as-Design（无 Figma）

本仓库 **不使用 Figma**。UI 阶段采用更严的 **Code as Design**：设计产物必须是 **可打开预览的代码**，不是静态稿平台。

## 三步（不变）

```text
① UX（文档/流程图）→ ux_status: accepted
② 实现行为（可 stub）
③ UI = 可预览代码 + 配置 → ui_status: accepted
```

## ① UX — 工具

| 用途 | 工具 |
|------|------|
| 正文 | Markdown：`doc/ux/<req-id>/` |
| 流程/状态 | **Mermaid** 画在 `flows.md` / `states.md`（GitHub/多数 Markdown 可渲） |
| 预览 UX 图 | VS Code/Cursor Markdown 预览，或 `doc/ux` 无需单独服务 |

UX **不要求**视觉预览；要求逻辑可评审。

## ③ UI — 强制 Code as Design

### 规定工具链（锁定）

| 层 | 技术 | 路径 |
|----|------|------|
| 设计源（可预览） | **HTML + CSS**（少量 JS 仅交互示意） | `ui/preview/<req-id>/index.html` |
| 设计令牌 | **CSS 变量** 与/或 `ui/*.toml`（同源语义） | `ui/tokens.css`、`ui/default.toml` |
| 本地预览 | 仓库脚本起静态服务 | `python scripts/ui_preview.py` |
| 映射说明 | Markdown | `doc/ui/<req-id>/mapping.md` |

**禁止**仅用不可运行的纯叙述通过 `ui_status: accepted`。  
**禁止**依赖 Figma/Sketch/Penpot 作为唯一真源（外链截图最多当附录）。

### `ui_status: accepted` 最低文件

```text
ui/preview/<req-id>/index.html    # 必须能浏览器打开并看到效果
doc/ui/<req-id>/brief.md
doc/ui/<req-id>/mapping.md        # 逻辑名 → DOM/data-attr/配置键
doc/ui/<req-id>/acceptance.md
```

`adl_check.py` 在 `ui_status=accepted` 时校验 `ui/preview/<req-id>/index.html` 存在。

### 预览怎么开

```bash
cd experiments/mcp_guard   # 或仓库根
python scripts/ui_preview.py
# 浏览器打开提示的 URL，例如 http://127.0.0.1:8765/REQ-TRAY-UI/
```

### 和真正产品壳的关系（UI as Config/Code）

```text
ui/preview/     ← 设计期真源（人眼预览）
ui/*.toml       ← 令牌/文案/槽位（运行时 ui_shell 读取）
src/ui_shell    ← 实现期：绑定 contracts + 读配置（可后于预览）
```

预览 HTML 用与 TOML **同一套 token 名**（在 `mapping.md` 写明），避免设计一套、实现另一套。

## 角色

| 角色 | UX | UI preview 代码 |
|------|----|-----------------|
| designer | `doc/ux/**` | `ui/preview/**`、`doc/ui/**`、`ui/*.toml` |
| module | ❌ | 只按 mapping 接壳；不改预览语义除非回 UI Issue |
| lead | accept | accept；可直推 |

## 为何不用 Figma

- 无账号/工具链依赖  
- Agent 可直接改 HTML/CSS  
- 与「UI as Config/Code」、O(1)（皮肤不进业务插件）一致  
- 预览 = 真浏览器，不是切图近似
