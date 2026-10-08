# Code-Spec V2 Update Guidelines

`spec.md` 与 `task.md` 在开发中是活的 Blackboard，但只因**新证据或目标状态变化**而更新。

## Update `spec.md` when

- 发现新的 VERIFIED path / symbol / test evidence；
- Open Question 得到足以改变/确认策略的新证据，或被明确接受为 assumption/risk；
- 现有 data/control flow 理解被推翻；
- Change Strategy 因证据改变；
- 新风险或 invariant 被真实证据揭示；
- Expected / Non-change Surface 发生有理由变化。

不要因为 Coder 完成了普通局部编辑就更新 spec。

## Update `task.md` when

- Milestone 在 `TODO / DOING / DONE / OBSOLETE` 之间发生有意义变化；
- 新证据改变依赖；
- Milestone 被 split / merge / reorder；
- Milestone 因现有 abstraction 或策略改变而 `OBSOLETE`；
- 独立验证产生可引用 evidence；
- 新 Unknown 阻塞当前 Goal。

## Never do

- 把 Coder 的临时 micro-plan 永久复制进 task.md；
- 为了保持旧计划“正确”而删除 contradictory evidence；
- 把未执行的命令写成验证证据；
- 将所有 source diff 逐行同步进 spec；
- 因为 task 数量变化就视为规划失败。

## Replan Record

只有 strategy / milestone topology 真正变化时记录：

```markdown
| Date | New evidence | Decision | Goal impact |
|---|---|---|---|
| ... | `path::symbol` revealed ... | M3 obsolete; reuse X | none |
```

日常 edit/test 循环不是 replan history。

## Consistency Check

每次显著 replan 后确认：

1. Goal Model 是否仍然有效？若目标本身变化，回到 `understanding` 获取用户确认。
2. `spec.md` 的重要结论是否区分 VERIFIED / INFERRED / ASSUMED / UNKNOWN？
3. Goal Board 是否仍描述 Outcome 而不是动作？
4. 新 strategy 是否仍满足 invariants / Non-Goals？
5. validation claims 是否仍能证明目标？
