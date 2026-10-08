---
name: understanding
description: 当用户意图、Desired Outcome 或高影响业务/架构决策仍不清楚时使用；已知行为的局部改动或明确 bug 不要用。
---

# Understanding V2 — Goal Model

## Purpose

把用户的自然语言需求转化为一个稳定的“意图与目标契约”，使后续所有 Agent 在局部实现时仍然能够回答：

- 为什么做？
- 当前到底哪里不够？
- 希望系统发生什么变化？
- 怎样才算实现得好？
- 哪些东西不应该被顺手改变？

本 Skill 不负责函数、类、文件级实现规划，也不替 `code-spec` 重建代码现实。

## Task Classification

### FAST_PATH

满足以下条件时，不生成完整 Goal Model：

- 预期行为明确；
- 影响范围局部；
- 不改变业务规则、数据模型、状态机、权限、计费、核心架构或主要用户流程；
- 修改可逆；
- 可以直接通过现有代码/测试验证。

例如：拼写、明确 bug、局部样式、小范围机械重构、补充已知行为测试。

### FULL_ALIGNMENT

出现任一情况时进入本 Skill：

- “为什么做”或“什么叫成功”不清楚；
- 新功能或核心流程改变；
- 需要决定状态、权限、数据归属、业务规则；
- 可能影响 API、数据库、状态机、并发、协议或架构边界；
- 用户给的是一个界面/技术手段，但真正目标尚未确认；
- 代码、测试、文档与用户描述相互冲突；
- 错误决策会造成明显返工。

## Core Principle

**目标提前确定，证据提前收集，约束提前明确，实现细节延迟决策。**

不要把“问得很细”误认为“理解得很深”。高质量 understanding 的核心是目标模型稳定，而不是异常分支数量最多。

## Required Inputs

开始前按需读取：

1. `context.md`：Project Mission、Strategic Goal、Glossary、Architecture Intent、Global Invariants、Non-Goals。
2. `DESIGN.md`：若涉及用户体验。
3. 相关历史需求或明确存在的契约。
4. 代码证据：只有当某个问题能通过代码回答时，优先交给 `explorer`，不要向用户询问代码事实。

## Goal Model

最终必须建立以下结构；不相关的维度可以省略，不为了模板完整强行填充。

### 1. Project Alignment

- Project Goal：本项目长期要解决什么。
- Current Strategic Goal：当前阶段最重要的系统演进方向。
- Contribution：本需求如何推动上述目标。

如果无法建立这一层，必须显式标记 `PROJECT_ALIGNMENT_UNKNOWN`，而不是假装每个功能天然有价值。

### 2. Observed Problem

描述当前现实，而不是直接描述方案：

- 谁遇到什么问题？
- 当前行为是什么？
- 证据来自哪里？
- 为什么现状值得改变？

### 3. Desired Outcome

描述完成后系统/用户应该处于什么状态。

优先使用结果语言：

- “用户能恢复失败操作且不丢数据”
- “多端口状态互相隔离”

避免把手段写成目标：

- “加一个 Toast”
- “新增 FooManager”

### 4. Solution Direction

只记录已经足够稳定的策略级方向，例如：

- 复用现有 port abstraction；
- 保持 core 与 vendor SDK 解耦；
- 使用既有状态机扩展，而不是创建平行状态机。

禁止在本阶段冻结函数签名、类名、具体文件修改列表，除非它们本身就是外部兼容契约。

### 5. What Good Looks Like

根据任务选择真正相关的质量维度：

- Functional correctness
- Architecture / separation of concerns
- Maintainability
- Simplicity
- Performance / latency / resource use
- Reliability / recovery
- Compatibility
- Security / privacy
- Observability
- UX / accessibility
- Testability

每个选中的维度必须写成可判断的结果，而不是“代码优雅”“体验良好”。

### 6. Invariants & Constraints

只记录真实存在的约束：

- 外部协议或兼容性；
- 已确认业务规则；
- 硬件/环境限制；
- 安全边界；
- 长期架构意图。

不要把通用最佳实践自动升级为本需求的不可违背红线。

### 7. Non-Goals

记录用户明确不做、延期或不应被本需求顺手扩展的内容。

### 8. Unknowns & Assumptions

对仍未解决的问题标记：

- `HIGH`：可能改变目标、状态模型、架构边界或主要实现策略，必须解决或由用户显式接受风险；
- `MEDIUM`：可以带着可逆假设继续，但必须可见；
- `LOW`：不阻塞推进。

## Evidence-first Clarification

不要默认 Grill 用户。

每遇到一个未知，按顺序判断：

1. `context.md` / 既有需求是否已回答？
2. 代码、测试、schema、配置是否能回答？若能，交给 Explorer。
3. 是否属于外部领域事实？若用户不熟悉且风险高，使用 `teach-me`。
4. 只有用户意图、偏好、业务决策本身才向用户提问。

提问时一次只处理一个真正影响决策的问题，并给出推荐与代价。

## Multi-Agent Use

复杂需求可并行派 Explorer 回答独立问题，例如：

- 当前行为与入口在哪里？
- 是否已有类似功能？
- 哪些测试定义了兼容行为？
- 现有 architecture intent 是否已经体现在代码中？

Explorer 返回 Evidence Packet；Primary 汇总后再决定是否还需要询问用户。

## Readiness Gate

使用以下状态之一：

### `GOAL_READY`

Project Alignment、Observed Problem、Desired Outcome、关键 `What Good Looks Like`、Invariants 和 Non-Goals 已足够明确；剩余未知不会改变目标本身。

允许存在可逆 assumptions，但必须直接记录在 Goal Model 中，不再为“带假设但可推进”创建额外 readiness 状态。

### `NOT_GOAL_READY`

仍有 HIGH unknown 会改变用户意图、核心状态模型、业务规则或成功定义。

不要因为小细节未知就阻止推进。

## Output

FULL_ALIGNMENT 的主要产物写入：

`user_plan/<feature>/<feature>.md`

使用 `references/demand-template.md`，但模板服务于表达，不是必须逐栏填满的表单。

该文档是 Intent / Goal Model，不是技术施工图。

## Exit Criteria

完成前确认：

- 能用一两句话解释为什么做；
- 能明确描述从 current state 到 desired state 的变化；
- `What Good Looks Like` 足以指导后续局部工程判断；
- 没有把未经验证的实现猜测写成 requirement；
- High-impact unknown 已解决或显式接受；
- Non-Goals 清楚；
- 后续 `code-spec` 可以从这里出发调查代码，而不需要猜用户意图。
