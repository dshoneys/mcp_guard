# Agent operating manual (MCP Guard)

You are working in **shinjiyu/mcp_guard** (local path often `experiments/mcp_guard` under kuroneko).

Read and follow:

1. [`doc/agents/ROLES.md`](doc/agents/ROLES.md)
2. [`doc/agents/WORKFLOW.md`](doc/agents/WORKFLOW.md)
3. [`doc/agents/UI-DESIGN.md`](doc/agents/UI-DESIGN.md)
4. [`doc/structurizr/ADL-RULES.md`](doc/structurizr/ADL-RULES.md)
5. Run `python scripts/adl_check.py` before finishing ADL/structure work

## Default role for this Cursor

**This Cursor workspace agent is `lead`.**

- May edit ADL and push `master` (on this PC use `hutao`)
- Owns UX/UI acceptance (`ux_status` / `ui_status` → `accepted`)
- Other agents: `designer` | `module` | `integrator` | `reviewer` — claim Issues, branch only

Only switch away from `lead` if the user explicitly assigns another role for a sub-task.

Framework checklist: [`doc/agents/FRAMEWORK-STATUS.md`](doc/agents/FRAMEWORK-STATUS.md).

## Scope lock (when acting as module — not default)

Only touch paths listed in the claimed Issue `component`. Do not edit other plugins. Do not edit `doc/structurizr/model/*` unless lead has already landed ADL on `master`. For UI features, **`ux_status` must be `accepted`** before implementation.
