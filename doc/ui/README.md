# UI packs — Code as Design（无 Figma）

Step ③. **真源是可预览的 HTML/CSS**，不是设计软件。

```bash
python scripts/ui_preview.py
# http://127.0.0.1:8765/REQ-TRAY-UI/
```

| 路径 | 用途 |
|------|------|
| `ui/preview/<REQ-ID>/index.html` | 必须能预览 |
| `ui/tokens.css` + `ui/default.toml` | 令牌（设计=配置） |
| `doc/ui/<REQ-ID>/` | brief / mapping / acceptance |

见 [`../agents/UI-DESIGN.md`](../agents/UI-DESIGN.md)。
