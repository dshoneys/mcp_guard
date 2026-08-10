# 1. Structurizr / ADL 为 MCP Guard 架构权威

## Status

Accepted

## Context

MCP Guard 需要多 Agent 协作开发，并落实两条工程不变量：

1. 业务插件出度 O(1)（禁插件互依）
2. 单测可蕴含集 \(R_U\) 与功能集 \(R\) 的差集 \(R_{\mathrm{manual}}=R\setminus R_U\) 必须显式，并走人测/环境验收

## Decision

- 以 `doc/structurizr/workspace.dsl` 为 **C4/ADL 权威**（人读 + Structurizr 工具）。
- 以 `doc/structurizr/model/graph.json` 为 **可运算依赖图**（出度 / NoCross）。
- 以 `doc/structurizr/model/requirements.json` 为 **功能集 \(R\)** 与测试种类权威。
- `scripts/adl_check.py` 为 CI/本地门禁：算 \(\max\mathrm{out}\)、横向边、\(R_{\mathrm{manual}}\)。
- 代码与模型漂移时，先改 ADL/model，再改实现（Structurizr-first）。

## Consequences

- 新需求必须先进入 `requirements.json` 并标明 `test_kind`。
- 新插件必须先进入 `graph.json` / DSL，且不得增加其它插件出度。
- `serve::run_with` 只依赖 contracts；`main`（cli）接线 `LoopbackScanner` / `SoftWatcher` / `JsonlSink`。
