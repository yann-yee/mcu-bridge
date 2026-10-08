# What Makes a Good Archive V2?

归档质量由“结果是否可证明、知识是否可复用”判断，而不是由历史 task 是否全部打勾判断。

## Outcome Audit

- [ ] 原始 Desired Outcome 有直接证据。
- [ ] `What Good Looks Like` 中相关维度已被验证或显式接受风险。
- [ ] 关键 invariants 保持。
- [ ] 必要 Milestone 为 DONE；不再必要的 Milestone 可以为 OBSOLETE，但必须有新证据与理由。
- [ ] 不存在隐藏的 HIGH-risk unknown。
- [ ] Test Runner / Reviewer / DESIGN_REVIEW 等独立证据可追溯。
- [ ] 没有通过删除测试、降低标准或悄悄改 Goal 来制造“通过”。

## Traceability

未来阅读归档的人应能回答：

```text
为什么做？
↓
当时的代码现实是什么？
↓
基于什么证据选择了这个策略？
↓
计划为什么发生过变化？
↓
最终用什么证据证明目标达成？
↓
学到了什么长期有效的东西？
```

因此 archive 应保留：

- Goal Model；
- feature design（适用时）；
- 最终 Evidence / Strategy；
- Goal Board 与重要 replan rationale；
- 关键验证证据或可追溯引用；
- residual risk；
- 有实际内容的 retrospective。

## Knowledge Hygiene

### 应进入长期文件

只有跨未来任务仍会改变正确决策的知识。

### 不应进入长期文件

- “Feature X 已完成”；
- 一次性的 debug 命令；
- 某个函数的临时实现细节；
- 为本次 workaround 添加的永久规则；
- 纯粹为了有更新记录而写的 history entry。

## 合法的 OBSOLETE

例：原计划 M4 “建立新的缓存层”，在 M2 调查时发现项目已有完全满足要求的 cache abstraction。

正确做法：

```text
M4 = OBSOLETE
Reason: existing abstraction X already provides required outcome; evidence: path::symbol
```

错误做法：为了让 checkbox 全绿仍然创建第二套缓存。

## Archive Result

只有 `OUTCOME_VERIFIED` 可以归档。

Residual / accepted risk 直接记录在归档中，不通过派生状态编码。

若证据不足或仍有必要目标未完成，返回 `BLOCKED` 并继续 delivery loop，而不是降低门槛。
