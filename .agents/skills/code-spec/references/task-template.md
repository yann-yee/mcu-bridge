# task.md — Lightweight Goal Board

用于 `user_plan/<feature>/task.md`。

`task.md` 是长任务的轻量执行黑板，不是完整工作流状态机，也不是执行日志。

```markdown
# <Feature> — Goal Board

## Anchor

- **Desired Outcome**:
- **Non-Goals**:

## Now

> 仅在 milestone 变化、replan、handoff 或 resume 时更新。

- **Focus**:
- **Last verified**:
- **Blocker / open issue**:
- **Next**:

## Milestones

| Milestone | Depends on | Done when | State |
|---|---|---|---|
| M1 — <outcome> | — | <observable evidence> | TODO |
| M2 — <outcome> | M1 | <observable evidence> | TODO |

States: `TODO / DOING / DONE / OBSOLETE`

## Open Issues

只记录会影响推进的问题。

| Issue | Impact | Next probe / decision |
|---|---|---|
| | HIGH / MEDIUM / LOW | |

## Final Evidence

- <claim> — <evidence / result>
```

## 使用规则

- Milestone 标题描述结果，不描述“修改哪个文件”。
- `READY` 不需要状态：依赖满足且值得做时，Primary 直接从 `TODO` 开始。
- `VALIDATING` 不需要状态：验证中的 Milestone 仍是 `DOING`。
- 阻塞不需要新增 Milestone 状态：把原因写入 `Now.Blocker` 或 `Open Issues`；无法继续时返回全局控制信号 `BLOCKED`。
- `DONE` 必须有证据。
- `OBSOLETE` 表示新证据证明该结果不再需要，不要为了 checklist 强行实现。
- 是否并行由依赖、写入边界和验证独立性现场判断，不在每个 Milestone 上预先维护额外字段。

## 长任务恢复

context compact / resume 后：

1. 读取 `Anchor + Now`；
2. 核对当前工作是否仍服务 Desired Outcome；
3. 如果信息不足，只补读必要的 Goal / Spec / Task 片段；
4. 如果新证据推翻 Strategy，返回 `REPLAN_REQUIRED`。

正常对齐时直接继续，不需要额外状态名。
