---
name: code-spec
description: 非平凡代码变更中，仅当现有行为、实现路径或边界仍不确定时使用；收集当前决策所需代码证据并形成最小 Change Strategy / Goal Board。
---

# Code-Spec V3 — Evidence & Change Strategy

## Purpose

在用户目标已经清楚，但**代码现实仍会影响实现决策**时使用。

不要因为“这是代码任务”就自动运行 Code-Spec。局部、明确、已有稳定实现路径的改动可以直接进入实现。

本 Skill 只回答：

- 当前目标依赖哪些真实代码事实？
- 哪些未知会改变策略？
- 最小且与现有架构一致的 Change Strategy 是什么？
- 需要哪些结果性 Milestone？

## Core Principle

**Evidence before Decision. Read only what can change the current decision.**

`spec.md` 是相关代码现实 + 策略；`task.md` 是轻量 Goal Board。二者都不是函数级施工剧本。

## Inputs

优先读取：

- 当前 Goal Model；
- 与本决策直接相关的项目约束；
- 已知的 feature/design artifacts。

不要默认重读整个 `context.md`、`DESIGN.md` 或仓库文档；只有当前决策需要时才读。

## 1. Frame the Decision Questions

每个调查问题都应能回答：

```text
Decision:
这个事实会改变什么决定？

Initial scope:
最浅层从哪里查？

Enough:
什么证据已经足够？
```

差的问题：

> 把 src/、SDK、kernel 都看一遍。

好的问题：

> 当前 port state 的 ownership 在哪里建立和释放？这个答案决定本次状态应该挂在哪一层。

## 2. Gather Bounded Evidence

从最接近当前项目的层开始。

只有当前层无法回答、且下一层信息**可能改变 correctness / strategy / validation** 时才继续下探。

如果必须跨重大边界，而该边界没有被当前任务授权，Explorer 返回：

`ESCALATION_REQUIRED`

Primary 决定是否继续。

停止探索，当：

- 当前策略已有足够直接证据；
- 剩余未知不会改变策略；
- 下一 probe 只增加背景知识；
- 调查开始研究基础设施本身，而不是当前 Goal。

复杂跨层调查、Explorer packet 和 scope ladder 见：

`references/evidence-and-planning-guide.md`

## 3. Reconstruct Only Relevant Reality

重要结论尽量指向：

- path / symbol；
- call/data flow；
- ownership / lifecycle；
- existing abstraction；
- test/config/runtime evidence。

重要结论按需要区分：

- `VERIFIED`
- `INFERRED`
- `ASSUMED`
- `UNKNOWN`

不要把推断写成事实。

未决问题只保留那些**仍可能改变 Change Strategy** 的 Open Questions。

## 4. Form the Smallest Sufficient Strategy

策略说明：

- 为什么选择这个方向；
- 复用哪些现有 abstraction；
- expected change surface；
- explicit non-change surface；
- 主要风险；
- 什么新证据会触发 replan。

实现细节尽量延迟到 Coder 的 JIT 判断。

新增 infrastructure / abstraction 前先问：

> 不建设它，当前 Outcome 是否仍然可达？

如果可达，优先更小方案；通用平台化想法放到 follow-up，不进入当前 feature。

## 5. Build a Lightweight Goal Board

`task.md` 只保留：

- Anchor：Desired Outcome / Non-Goals；
- Now：Focus / Last verified / Blocker / Next；
- 结果性 Milestone；
- Open Issues；
- Final Evidence。

Milestone 状态：

`TODO / DOING / DONE / OBSOLETE`

依赖满足即可开始，不需要额外 READY/VALIDATING 状态。

模板：

- `references/spec-template.md`
- `references/task-template.md`

## Reviewer Use

只有当策略本身存在高影响架构、并发、状态、协议、安全、迁移或兼容性风险，且独立 critique 能改变决策时，才在实现前使用 Reviewer。

不要把 Reviewer 作为 Code-Spec 固定尾步骤。

## Exit Criteria

Code-Spec 足够进入 delivery，当：

- 关键实现决策有足够代码证据；
- 高影响 Open Question 已解决、接受或明确阻塞；
- Change Strategy 服务原始 Goal 且边界清楚；
- 不变量来自真实证据；
- Goal Board 描述结果而不是函数级动作；
- Coder 仍有足够局部自主空间。

需要更详细的 exploration、invariant、risk、parallel discovery 或 replan 规则时，再读取：

`references/evidence-and-planning-guide.md`
