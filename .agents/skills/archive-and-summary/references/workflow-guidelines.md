# Archive Workflow Guidelines

本指南只补充 Archive & Summary 的执行细节，不建立第二套交付状态机。

## 1. Outcome First

归档前只确认：

- Desired Outcome 是否有直接证据；
- 关键 invariants 是否保持；
- 必要 Milestone 是否 DONE；
- 独立验证 / review 是否满足当前风险；
- residual risk 是否已记录。

不要要求历史上所有 Milestone 都 DONE；`OBSOLETE` 是合法结果。

如果证据不足，返回 `BLOCKED`，继续 delivery loop。

## 2. Independent Audit When Useful

高风险或复杂交付 SHOULD 使用独立 Reviewer / 只读 Agent 做 Outcome Audit。

简单、低风险且已有充分独立测试证据的交付，不要为了流程形式强制再创建一个审计 Agent。

审计的是 Goal 与 Evidence，不是 checkbox 数量。

## 3. Physical Archive

Outcome verified 后，将：

`user_plan/<feature>/`

移动到：

`user_plan/archive/<feature>/`

使用当前环境中安全、可验证的文件操作方式；不要硬编码某一种 shell 或操作系统命令。

移动前：

- 确认源目录与目标目录；
- 避免覆盖已有 archive；
- 不把临时 build/log 产物当作 feature artifact 保存。

## 4. Long-term Learning

归档本身**不自动**修改 `context.md`、`DESIGN.md` 或 `AGENTS.md`。

只有当本次交付产生跨未来任务仍会改变正确决策的知识时，才更新对应长期文件。

例如：

- 新的全局 invariant → context.md；
- 跨功能 UX rule → DESIGN.md；
- 新的稳定工程/安全边界 → AGENTS.md。

“Feature X 已完成”不是长期知识。

## 5. User-facing Completion

最终只需要简洁说明：

- Outcome 是否已验证；
- 关键验证证据；
- residual risk（如有）；
- archive 位置（如实际归档）。

不要为了仪式生成空洞复盘或“完美完成”式措辞。
