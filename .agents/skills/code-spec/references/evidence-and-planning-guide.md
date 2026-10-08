# Code-Spec Evidence & Planning Guide

仅在简单 Code-Spec 根规则不足时读取。

## Bounded Exploration

可用的跨层参考：

```text
current project
→ direct dependency / public API
→ framework / OS internals
→ firmware / bootloader
→ register / silicon
```

这不是必走路径。只有当前层证据指向下一层且下一层会改变决定时才下探。

### Explorer Assignment

给 Explorer：

- Question
- Decision affected
- Initial scope
- Stop condition
- Explicit non-goals

推荐 Evidence Packet：

```markdown
## Question
## Findings
- path::symbol — fact

## Relationships
## Confidence
## Unknowns
## Risks
## Recommended Next Probe
```

按问题并行，不按目录平均切分。

## Open Questions

只保留会改变 Strategy 的问题：

| Question | Impact | Next evidence / accepted assumption |
|---|---|---|
| | HIGH / MEDIUM / LOW | |

问题得到足够答案后，把稳定结论移入 Evidence Map，不维护复杂生命周期状态。

## Relevant Reality Checklist

按当前任务需要选择：

- entry / control flow；
- data flow；
- state ownership；
- lifetime；
- concurrency；
- external contract；
- persistence；
- existing tests；
- validation surface；
- existing abstraction；
- compatibility constraints。

无关项不要为了模板完整而调查。

## Evidence-derived Invariants

Invariant 必须来自实际依赖：

```text
Evidence:
existing callers/tests depend on -EAGAIN.

Invariant:
this change must preserve -EAGAIN semantics.
```

不要把通用最佳实践升级成 feature-specific hard rule。

## Infrastructure Restraint

新增通用能力前问：

1. 当前 Goal 是否必须依赖它？
2. 项目/注册资源是否已有能力？
3. 能否用更小局部适配完成？
4. 新基础设施是否扩大验证/维护面？
5. 不建设它，Outcome 是否仍可达？

优先 smallest sufficient capability。

## Replan Triggers

典型触发：

- 真实代码与 Evidence Model 冲突；
- 已有 abstraction 使当前策略不再合理；
- 必须突破关键 invariant；
- 原以为独立的目标实际强耦合；
- 测试揭示需求/状态模型错误；
- 并发、生命周期、安全、性能或兼容风险显著变化；
- 更小且更可验证的方案出现。

出现这些情况，更新策略；不要为了保住旧 spec 强迫代码迁就。

## Parallel Discovery / Implementation

Explorer 可以按独立问题并行。

Coder 只有在：

- 目标语义独立；
- 写入/集成边界稳定；
- 一个局部决定不会迫使另一个猜测；
- 每项能独立验证；

时才并行。

文件不重叠本身不是充分条件。
