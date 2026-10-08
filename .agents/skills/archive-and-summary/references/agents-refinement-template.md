# Agent Refinement — Evidence-Driven Learning

用于复杂交付结束时，把真实“痛苦记忆”或高价值收益转成未来可执行的改进。

核心原则：

> 有实证才进化；能预防复发才写规则。

不要因为进入 archive 就自动修改 AGENTS。

## 1. Identify Real Friction

只找具体事件：

- 同类错误是否重复出现？
- 用户是否明确说“以后不要再这样”？
- 哪个等待/测试最终没有改变 handoff？
- 哪个 manual workaround 重复了？
- 哪个项目事实如果更早知道，可以明显减少返工？
- 哪个问题其实来自 Core/Domain 方法，而不是当前项目？

## 2. Classify Before Writing

### PROJECT_RULE → AGENTS.md

跨多个未来任务都会遇到，并存在明确预防行为。

例：

> 设备写测试只能有一个 writer；其它 Agent 只能做只读采集，避免配置互相覆盖。

### PROJECT_MEMORY → context.md

长期架构事实、业务语义、global invariant。

### FEATURE_ONLY → spec/task/archive

只属于当前 feature 的故障、临时 workaround、特定版本现象。

### RUNBOOK → 对应 README / 操作文档

登录、烧录、设备恢复、测试拓扑等可重复操作知识。

### CORE_CANDIDATE / DOMAIN_CANDIDATE

问题来自 Agent routing、validation strategy、learning/persistence 或通用领域判断。

记录：

~~~text
Observed failure:
Why current method allowed it:
Generalizable scope:
Proposed rule/change:
Evidence that this is recurring or costly:
~~~

普通项目 feature 不应擅自改中央 Core/Domain Pack；只有用户明确要求改工作流，或当前仓库本身就是对应 Pack 时直接修改。

## 3. Write the Smallest Preventive Rule

好规则应包含：

~~~text
Trigger / situation
→ required or forbidden behavior
→ why / failure it prevents
~~~

避免：

- “加强沟通”
- “提高质量”
- “注意性能”
- 一次性 workaround 永久化
- 把完整调试故事复制进 AGENTS

## 4. Remove Stale Rules Too

学习不只是添加。

如果新版 Core/Skill 已经承担某条通用流程，或项目事实已经改变：

- 删除重复规则；
- 删除冲突规则；
- 保留真正 project-specific 部分。

这对 copy-overlay 项目尤其重要。

## 5. Immediate Learning

明确的 durable 用户纠正或重复 costly failure 不需要等 archive。

命中 spec-workflow 的 Operational Learning Gate 时，应立即写到最小正确位置；archive 只负责复核它是否过宽、过时或应该升级为 Core/Domain candidate。
