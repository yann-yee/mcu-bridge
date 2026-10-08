# What Makes a Good Code-Spec V2?

`spec.md` + `task.md` 的质量不由“是否足够详细”判断，而由它们能否让多个 Agent 在不共享完整上下文的情况下，基于同一目标和证据做出一致工程判断来判断。

## spec.md 质量标准

- [ ] **Goal Anchored**：每个关键证据和策略都能解释与目标的关系。
- [ ] **Evidence Traceable**：重要结论尽可能引用真实 path、symbol、test、config 或运行证据。
- [ ] **Reality Reconstructed**：关键 control/data flow、ownership、state/lifetime 已覆盖到足以做架构决策的程度。
- [ ] **Open Questions Visible**：会改变策略的问题保持可见；证据不足的结论标成 UNKNOWN，而不是补全故事。
- [ ] **Invariants Evidence-derived**：feature 红线来自当前系统真实依赖，不是模板化最佳实践。
- [ ] **Strategy-level, not Function-level**：策略说明边界、方向和取舍，不冻结局部函数实现。
- [ ] **Replan-aware**：说明什么新证据会让当前策略失效。
- [ ] **Bounded**：只调查改变本次决策所需的代码，不追求全仓百科。

## task.md 质量标准

- [ ] **Outcome-oriented**：每个 Milestone 描述系统状态变化，而不是编辑动作。
- [ ] **Completion Evidence 明确**：能说明怎样客观证明 Milestone 已完成。
- [ ] **Dependencies 真实**：依赖由语义和架构决定，不按文件顺序排列。
- [ ] **Parallelism 谨慎**：是否并行由真实依赖、写入面和验证独立性判断，不需要为每个 Milestone 维护额外标志。
- [ ] **Risk Visible**：高风险问题在 Open Issues / Spec Risks 中可见并触发相应审查。
- [ ] **Replanning Legal**：允许 split / merge / reorder / obsolete，并记录基于什么新证据变化。
- [ ] **No Fake Commands**：不提前杜撰尚未验证存在的测试命令。

## 反模式

### 预测未来代码

```text
在 src/foo.c 新增 foo_init(ctx)
在 src/bar.c 调用 foo_init(ctx)
```

如果这些结构还没有被代码现实强制要求，就属于过早承诺。

### Best-practice Dump

把“所有写 API 都必须幂等”“所有外设失败都必须复位”等通用规则写进每个 feature spec，会稀释真正的不变量。稳定通用规则应进入 AGENTS.md；feature spec 只记录本次相关且有来源的限制。

### Directory-based Exploration

```text
Explorer A 看 src/
Explorer B 看 drivers/
```

应改成问题导向：

```text
Explorer A：RX 到 state machine 的完整路径是什么？
Explorer B：timestamp ownership 与 completion path 是什么？
```

### Checklist Completion Bias

“所有 task 都 DONE”不是质量证明。Milestone 可以因新证据变为 OBSOLETE；真正要验证的是原始 Goal 与 What Good Looks Like。

## Ready for Delivery

当一个新的 Coder 只读取 Goal Model、`spec.md` 和分配给它的 Milestone 后，应该：

- 明白 why；
- 明白代码现实中哪里相关；
- 明白哪些边界不能突破；
- 有足够自主空间决定局部 how；
- 知道什么情况应该返回 `REPLAN_REQUIRED`；
- 知道什么证据才能证明结果成立。

满足这些条件，才是高质量 code-spec。
