# MCP Guard Structurizr / ADL

## Authority

| Artifact | Role |
|----------|------|
| [workspace.dsl](./workspace.dsl) | C4 model (human + Structurizr) |
| [model/graph.json](./model/graph.json) | Checkable dependency graph |
| [model/requirements.json](./model/requirements.json) | Feature set \(R\) |
| [ADL-RULES.md](./ADL-RULES.md) | O(1) + \(R_{\mathrm{manual}}\) rules |
| [modules-catalog.md](./modules-catalog.md) | Module contracts |
| [COMPONENT-TEST-MAP.md](./COMPONENT-TEST-MAP.md) | Req ↔ test map |
| [decisions/](./decisions/) | ADRs |

## Check

```bash
python scripts/adl_check.py
```

Exit 0 only if: plugin \(\mathrm{out}\le K\), no plugin→plugin edges, no `test_kind: none`, and every `unit`/`contract` req has a non-empty `tests` list.

Prints \(R_{\mathrm{manual}}\) for human QA.

## Multi-agent roles

See [`../agents/ROLES.md`](../agents/ROLES.md) and [`../agents/WORKFLOW.md`](../agents/WORKFLOW.md).  
Root [`AGENTS.md`](../../AGENTS.md): **lead** may push `master`; others claim Issues and use branches only.

## Formalism

See also [../formalism-o1-ut.html](../formalism-o1-ut.html).
