---
name: context-of-user
description: 仅在创建或更新长期 context.md 时使用；保存会影响未来多个任务决策的项目级知识，不记录 feature 流水账。
---

# Context-of-User V2 — Long-term Project Memory

## Purpose

让未来零上下文 Agent 能快速恢复项目“为什么存在、往哪里走、哪些原则不能破坏”的长期认知。

`context.md` 不是开发日志，也不是所有 feature 文档的摘要。

## Memory Admission Test

每条准备写入 `context.md` 的信息必须通过：

> 如果未来开始一个完全不同的 feature，这条信息仍然可能改变 Agent 的正确决策吗？

如果答案是否，就留在 feature archive、代码或 Git 历史中。

## What Belongs Here

### Project Mission

项目长期解决的问题、核心用户/系统价值。

### Current Strategic Goal

当前阶段跨多个 feature 的演进方向。不要把单个 feature 待办列表放进来。

### Success Model

项目层面的长期质量标准，例如可靠性、资源预算、兼容性或核心 UX 目标。

### Glossary

用户/业务特有术语、缩写、角色和语义。

### Architecture Intent

长期架构方向，例如：

- core 与 vendor SDK 解耦；
- 共享数据与 per-port state 的 ownership 原则；
- 哪层负责协议、硬件、业务边界。

这不是当前源码目录清单，而是即使代码重构后仍应成立的设计意图。

### Global Invariants

跨 feature 都必须保持的行为或约束。

### Major Decisions

长期决策应包含：

- Decision
- Rationale
- Alternatives rejected
- Conditions that could invalidate the decision

### Non-Goals

明确长期不做或当前战略阶段排除的方向。

### Strategic Unknowns

尚未解决但未来会影响多项工作的高价值问题。

## What Does NOT Belong Here

- Feature X 已完成；
- 某次归档时间；
- task checkbox 状态；
- 单一函数实现；
- 临时 bug workaround；
- 一次性测试日志；
- Agent 的普通建议；
- 可以直接从代码稳定推导出的低价值事实。

## Evidence and Status

长期信息按需要区分：

- `CONFIRMED`：用户明确确认或有稳定项目证据；
- `CURRENT_DIRECTION`：当前战略/架构方向，未来可能演进；
- `DRAFT`：尚待用户确认；
- `OPEN`：战略 Unknown。

不要把模型推荐静默写成 `CONFIRMED`。

## Update Policy

### Add

只有 Memory Admission Test 通过时新增。

### Modify

新证据或用户决定推翻旧共识时，优先更新原条目，并保留必要 rationale；不要简单追加相互冲突的新段落。

### Remove / Retire

旧规则明确失效时标记 retired 或更新为当前状态，避免未来 Agent 同时读到两套有效规则。

## Interaction with Other Artifacts

- feature goal 只属于单需求：放 `<feature>.md`；
- feature implementation evidence：放 `spec.md`；
- feature milestone：放 `task.md`；
- 跨功能 UX / visual rule：放 `DESIGN.md`；
- 工程命令、安全、工具与环境：放 `AGENTS.md`；
- 长期 Project Mission / Architecture Intent / business consensus：放 `context.md`。

## Multi-Agent Use

Primary 可让 Explorer 检查“候选长期规则是否已经存在、是否与代码/历史共识冲突”，但长期业务和架构意图的最终写入由 Primary 整合，不能让多个 Worker 并行修改 `context.md`。

## Output

使用 `references/context-template.md`。

模板不是必须每次更新所有章节；采用最小、增量、高信噪比修改。

## Exit Criteria

更新后的 `context.md` 应满足：

- 新 Agent 能快速理解 Mission 与战略方向；
- 长期 Architecture Intent 与 Global Invariants 清楚；
- 没有 feature 完成流水账；
- 没有把实现细节当长期知识；
- 新旧决策不自相矛盾；
- DRAFT / OPEN 与 CONFIRMED 可区分；
- 内容比 Git log 更像“项目长期记忆”，而不是另一个 Git log。
