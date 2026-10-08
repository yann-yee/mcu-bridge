---
name: spec-workflow
description: 已有 Goal/spec/task 且需要多步交付时使用；按最新证据选择下一动作并按需调度 worker。局部明确改动不必加载。
---

# Spec-Workflow V4 — Adaptive Delivery

## Purpose

把已明确的 Goal 和 Change Strategy 推进到有证据支持的完成状态。

本 Skill 是轻量控制器，不是固定流水线。局部、明确、可直接实现和验证的改动，不需要为了“流程完整”加载本 Skill。

## Primary Responsibility

Primary 只需要持续掌握：

- Desired Outcome；
- 当前 Focus；
- 最新关键 Evidence；
- 当前约束 / Non-Goals；
- 下一项真正有价值的决定。

不要把所有 Worker 的原始搜索、编辑、日志和测试细节留在 Primary 上下文。

## Agent Routing

**Use a worker because it owns an unresolved need, not because its topic is related.**

- Explorer：具体事实会改变当前决定；
- Coder：Goal、约束和 change boundary 已足够清楚；
- Test Runner：meaningful claim 缺少新的 runtime/integration 独立证据；
- Reviewer：存在 consequential static / architecture / risk uncertainty。

Additional Agent Work 必须同时满足：

1. **Information Value**：能增加当前没有的有效证据；
2. **Decision Value**：结果可能改变下一项真实决定。

已有可信证据足够时，不为了形式重新调用 Worker。

## Critical Path Gate

**有信息价值，不等于应该阻塞下一次 handoff。**

准备启动额外 exploration / validation / review 时问：

~~~text
Can this evidence change the next meaningful handoff decision?
~~~

如果能改变，例如：

- 当前源码是否可以交给用户编译；
- 是否可以合并/发布；
- 是否需要 replan；
- 是否可以接受关键 Outcome；

则它可以位于 critical path。

如果不能改变下一 handoff，只是：

- extra mock coverage；
- optional diagnostics；
- future optimization；
- 重复证明已有等价 evidence；
- 不影响当前 acceptance 的扩展调查；

则应 **parallelize / defer / record GAP / stop**，不要继续阻塞用户。

## Adaptive Loop

~~~text
choose highest-value unfinished outcome
             ↓
enough evidence?
   ┌─────────┴─────────┐
  no                  yes
   ↓                    ↓
targeted explorer    implementation batch
   │                    ↓
   └────────────→ cheap local checks
                        ↓
              coherent change boundary
                        ↓
                 validation plan
            ┌───────────┼───────────┐
            ▼           ▼           ▼
         target       runtime      review
         checks       evidence     if useful
            └───────────┼───────────┘
                        ↓
               collect all evidence
                  expected / surprise
                        ↓
                accept / batch-fix /
                     replan
~~~

## Selecting the Next Action

不要机械选择 task.md 下一行。

优先考虑：

- dependency；
- uncertainty reduction；
- user/outcome value；
- risk；
- 是否能解锁下游；
- 是否已有足够 evidence；
- 是否位于真正 critical path。

## Coder Handoff

给 Coder 足够的结果和边界：

~~~text
Goal
Why
Relevant evidence
Constraints / invariants
Success evidence
Expected change surface
Explicit non-goals
~~~

不要把 Primary 猜测的函数级施工步骤写成命令。

Coder 自行形成 JIT micro-plan，并在实现中做便宜 local checks。local checks 用于尽早发现明显错误，不等于每次小 edit 都触发独立 validation。

## Batch Validation

默认按**语义完整的实现批次**验证，而不是“改一点、测一点”。

### Before the batch

先明确：

- 必需检查；
- 可选补充；
- 哪些可并行；
- 哪些存在真实数据依赖；
- 哪些 failure 会阻断下一 handoff。

### During implementation

- 使用便宜 local checks 防止明显错误扩散；
- 准备必要的已有测试入口；
- 不在每个 helper/file 后单独派 Test Runner；
- 不临时扩建大型测试框架阻塞尾部交付。

### At the validation boundary

无真实依赖的 static/build、host/runtime validation、Reviewer POST 应并行执行。

一轮尽量收集全部独立失败后再统一修复。

修复后：

- 复用未变化代码的 fresh evidence；
- 只重跑受影响 claim；
- 再补必要的 integration boundary。

不要因为一个独立失败就立刻停止其它仍能产生价值的并行检查。

## Validation Value Gate

启动 Test Runner 前问：

~~~text
What claim needs independent proof?
What new evidence would this run add?
Can the result change acceptance / replan / next handoff?
Is existing fresh evidence already sufficient?
~~~

优先独立验证：

- 高风险 security / transaction / concurrency / protocol / persistence / migration / recovery；
- bug fix 需要独立复现；
- local check 无法证明真实 runtime/integration behavior；
- 准备接受关键 Outcome，但现有证据不足。

不要为中间施工步骤、文档/命名修改、刚刚跑过且代码未变化的等价确定性检查单独派 Test Runner。

## No Idle Waiting

Worker 运行时：

~~~text
Does its result block the next decision?
   ├─ yes → wait or do only independent useful work
   └─ no  → advance another useful action
             and stop/defer the worker once its evidence is no longer needed
~~~

不要为了“既然启动了就等它结束”消耗 critical path。

## Replan

如果新 evidence 表明当前 Strategy 不再成立：

\`REPLAN_REQUIRED\`

Primary 更新必要的 spec/task 信息并重新决定，不把 replan 视为失败。

## Operational Learning Gate

用户反馈和流程摩擦本身也是 Evidence。

当出现以下情况之一：

- 用户明确说“以后… / 不要再… / 记得… / 每次都… / 已经说过…”；
- 同类 costly failure / workaround 重复出现；
- Agent 的执行方式明显浪费时间或反复制造相同返工；

立即问：

> 如果未来 Agent 不看到这条经验，它是否很可能重复同样错误？

如果是，不必等到 feature archive，立即把规则写到**最小正确位置**：

- 项目长期执行/环境规则 → \`AGENTS.md\`；
- 长期项目事实/不变量 → \`context.md\`；
- 当前 feature 特有证据/坑 → \`spec.md\` / \`task.md\`；
- 设备/脚本操作方式 → 对应 runbook / README；
- 跨项目 Core/Domain 方法问题 → 在当前 feature learning 中标记为 Core/Domain improvement candidate。

不要创建万能 \`LESSONS.md\`，也不要把一次性 workaround 升级为永久法典。

## Long-task Recovery

context compact / resume / handoff 后：

1. 先读当前 feature 的 \`task.md\` Anchor + Now；
2. 核对当前 Focus 是否仍服务 Desired Outcome；
3. 只补读它明确引用的 Goal/spec/evidence；
4. Strategy 被新证据推翻才 replan。

**One Active Truth**：当前 feature 以 Goal Model + \`spec.md\` + \`task.md\`（以及适用时 \`design.md\`）为权威状态。历史诊断/validation 文件可以保留原始证据，但不应迫使恢复时重新猜“哪个文档最新”。

不要重新扫描仓库、重跑已完成调查，或按日期遍历所有历史 repair 文档。

## Parallelism

只并行真正独立的工作。

当 runtime validation 与 static/risk review 都能增加 decision value 时，可以并行 Test Runner + Reviewer；不要因为“实现完成”自动 fan-out。

## Blackboard Updates

只在信息真的变化时更新：

- \`spec.md\`：新事实、策略、风险；
- \`task.md\`：Milestone / Now / Final Evidence；
- Goal Model：只有用户目标本身变化；
- 长期 artifacts：通常在归档判断；但命中 Operational Learning Gate 的明确 durable correction 应立即沉淀。

避免每轮 repair/retest 都新建平行“current truth”文档。

## Outcome Gate

完成意味着：

- Desired Outcome 有客观证据；
- relevant What Good Looks Like / invariants 满足；
- 必要 Milestone DONE，失效 Milestone 可 OBSOLETE；
- 高影响 unknown 已解决或显式接受；
- consequential claims 有足够 validation；
- 高风险 change 在需要时获得独立 review；
- residual risk 明确。

不要把 checkbox 全绿、Worker done、或验证数量多等同于完成。

复杂的 risk router、Worker contract、failure policy、Git checkpoint 与 validation batching 示例仅在需要时读取：

\`references/delivery-details.md\`
