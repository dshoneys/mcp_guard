# REQ-MAIN-UI — information architecture

## Principle

**Risk detail list is the product.** Chrome (brand, status, counts, actions) stays compact; the list owns remaining viewport height.

## Home layout (top → bottom)

1. **Topbar** — small mark + “MCP Guard” + status chip (severity color)
2. **One-line summary** — headline + compact counts + short scan time
3. **Risk stage (flex:1)** — titled list; scrolls; empty state only when no items
4. **Toolbar** — Scan | Audit | Mute | vault link (secondary)
5. **Foot meta** — one tiny line (tray + audit basename)

## Vault

Secondary view (unchanged): back + form + names-only list.
