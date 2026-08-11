# MCP Guard ADL 规则

## 1. 出度 O(1)

对业务插件集合 \(P\)（`graph.json` 中 `role: plugin`）：

\[
\forall p\in P:\ \mathrm{out}(p)\le K
\qquad (K=2\ \text{默认：contracts + 可选 infra})
\]

\[
\forall p_1,p_2\in P:\ (p_1,p_2)\notin E
\]

插件出边只允许指向 `contracts` 或 `infra`（如 audit sink 端口）。

**不**约束：`contracts` / `compose` 的入度（允许 \(O(n)\)）。

### 3a. Vault NoContext（密钥）

Vault MCP **禁止**在 tool result 中返回明文密钥。权威说明：[`VAULT-NOCONTEXT.md`](./VAULT-NOCONTEXT.md)。

## 2. 单测集与功能差集

\[
R_U=\{r\in R\mid r.\mathrm{test\_kind}\in\{\mathrm{unit},\mathrm{contract}\}\ \land\ r.\mathrm{tests}\neq\emptyset\}
\]

\[
R_{\mathrm{manual}}=R\setminus R_U
\]

| test_kind | 含义 | 收工方式 |
|-----------|------|----------|
| `unit` | 纯逻辑，单测蕴含功能 | Agent / CI |
| `contract` | 端口契约测 | Agent / CI |
| `manual` | 环境/真机/人工 | 人测清单 |
| `none` | 尚未分配 | **门禁失败** |

`adl_check.py` 打印 \(R_{\mathrm{manual}}\)；若存在 `test_kind: none` 或 unit/contract 但 `tests` 为空 → fail。  
`ui:true` 必须走 **UX → 实现 → UI**：`ux_path`/`ux_status`、`ui_path`/`ui_status`、`ui_impl`（config|code|hybrid|none）。  
见 [`../agents/UI-DESIGN.md`](../agents/UI-DESIGN.md)。

## 3. 变更顺序

1. 改 `requirements.json` / `graph.json` / `workspace.dsl`
2. 跑 `python scripts/adl_check.py`
3. 再改 Rust 代码与测试
4. 更新 `COMPONENT-TEST-MAP.md` / `modules-catalog.md`
