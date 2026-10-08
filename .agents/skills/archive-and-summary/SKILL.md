---
name: archive-and-summary
description: 仅在 feature 交付结束并需要 Outcome Audit、归档或长期知识提炼时使用。不要用于普通中间进度总结。
---

# Archive & Summary V3 — Outcome Audit & Learning

## Purpose

回答三个问题：

1. 我们真的达成了原始目标吗？
2. 这次交付产生了什么值得未来 Agent 长期知道的新知识？
3. **这次哪里浪费了时间、重复踩坑或暴露了工作流缺陷，下一次怎样更便宜？**

归档是结果审计与学习，不是 checkbox 仪式。

## Inputs

- \`<feature>.md\` — Goal Model
- \`design.md\` — Experience Intent（适用时）
- \`spec.md\` — Evidence & Change Strategy
- \`task.md\` — Goal Board
- implementation diff / current code
- validation evidence
- reviewer findings（适用时）
- 用户在交付过程中的明确纠正/反馈

## Outcome Audit

### Goal Achievement

确认：

- Desired Outcome 是否真实出现；
- 原始 Observed Problem 是否被解决；
- 有没有 scope drift。

### What Good Looks Like

只审核本需求相关质量维度。每个结论必须指向代码、测试、运行态或 review evidence。

### Invariants

确认 Goal/spec 中关键 invariant 未被破坏。

### Goal Board

允许：

- \`DONE\`：有验证证据；
- \`OBSOLETE\`：有新证据说明不再需要。

阻塞：

- 必要但未 DONE 的 Milestone；
- 无 evidence 却标 DONE；
- 为迁就实现偷偷改变 Goal。

### Residual Risk

记录未验证场景、环境限制、accepted risk 和未来事项。

## Audit Result

只保留：

- \`OUTCOME_VERIFIED\`
- \`BLOCKED\`

不要派生更多 workflow state。

## Learning Extraction

每条候选知识先问：

> 未来一个不同 feature 开始时，这条信息是否仍可能改变正确决策？

### context.md

仅长期 Project Mission / Strategic Goal / Architecture Intent / Global Invariant / durable decision / Non-Goal / Strategic Unknown。

### DESIGN.md

仅跨功能稳定 Experience / UX / Visual System。

### AGENTS.md

仅项目长期执行会反复遇到的：

- 环境事实；
- build/test/run 方法；
- 工程约束；
- 安全边界；
- **重复踩坑且能由明确规则预防的问题**。

### feature artifacts / runbook

当前 feature 特有 evidence 留在 spec/task/archive；设备或脚本操作坑进入对应 README/runbook。

### Core / Domain improvement candidate

如果问题不是这个项目特有，而是：

- Agent routing；
- validation strategy；
- learning/persistence；
- 某领域通用判断；

则在 feature learning 中明确标记 \`CORE_CANDIDATE\` 或 \`DOMAIN_CANDIDATE\`，说明真实 failure case 和建议方向。

普通 feature 不应擅自修改安装的 \`.agents/\` / Core Skill；只有用户明确要求改工作流或当前仓库本身就是该 Skill/Core 时才直接修改。

## Process Friction Audit

不要只复盘技术 bug。主动检查：

- 哪段等待明显位于 critical path，但最终没有改变 handoff？
- 哪些 Test/Review/Explorer 重复了已有 evidence？
- 有没有“改一点、测一点”造成验证碎片化？
- 哪些失败本可以一次收集后批量修？
- 哪个 Worker 启动后只是因为“已经启动”而被等待？
- 用户在哪里纠正了执行方式？
- 哪个 manual workaround 重复出现？
- 恢复上下文时，是否因为历史 artifact 太多而重新猜 current truth？
- 哪条规则如果更早存在，可以显著减少本次时间或返工？

只记录有具体 failure/evidence 的结论。

## Immediate vs Archive Learning

明确用户纠正或重复 costly failure 不必等归档；\`spec-workflow\` 的 Operational Learning Gate 应当即时沉淀。

Archive 负责再次检查：

- 是否已经写到正确位置；
- 是否写得过宽；
- 是否应升级为 Core/Domain candidate；
- 是否有过时规则可以删除。

## Artifact Hygiene

当前 feature 的 active truth 应保持为：

~~~text
Goal Model
spec.md
task.md
design.md (when relevant)
~~~

原始日志、测试结果和历史诊断可以保留，但不要每轮 repair/retest 都创建新的“当前计划”文档。

如果存在大量历史报告，确保 spec/task 明确指向当前权威 evidence，避免 resume 时按日期重新推断。

## Physical Archive

Outcome Verified 后，将：

\`user_plan/<feature>/\`

整体移动至：

\`user_plan/archive/<feature>/\`

保留可追溯链：

~~~text
Why → Evidence → Decision → Change → Validation → Learning
~~~

归档移动本身不强制更新 context/AGENTS。

## Exit Criteria

完成归档，当：

- Outcome 为 \`OUTCOME_VERIFIED\`；
- 必要 Goal 完成；
- residual risk 明确；
- evidence 可追溯；
- durable learning 已选择性沉淀；
- Process Friction Audit 已检查真实成本；
- Core/Domain candidate 有真实 failure case 才记录；
- archive 中仍能明确识别 active truth 与历史证据。
