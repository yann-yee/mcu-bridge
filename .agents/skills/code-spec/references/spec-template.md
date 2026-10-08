# spec.md — Implementation Evidence & Strategy Template

用于 `user_plan/<feature>/spec.md`。

```markdown
# <Feature> — Implementation Evidence & Strategy

## 1. Goal Anchor

- **Source Goal Model**: `<feature>.md`
- **Desired outcome**:
- **Relevant What Good Looks Like**:
- **Non-Goals**:

## 2. Open Questions

只保留会改变当前策略的问题。

| Question | Impact | Evidence / next probe / accepted assumption |
|---|---|---|
| | HIGH / MEDIUM / LOW | |

已解决问题把结论移入 Evidence Map，不维护额外“已回答”状态。

## 3. Evidence Map

| Area | Path | Symbol / test | Finding | Classification | Confidence |
|---|---|---|---|---|---|
| | | | | VERIFIED / INFERRED / ASSUMED / UNKNOWN | HIGH / MEDIUM / LOW |

## 4. Current Architecture

只描述与本目标相关的实际架构。

```text
entry
  ↓
...
```

## 5. Current Control / Data Flow

```text
...
```

## 6. Ownership / State / Lifetime

- **Owner**:
- **Creation**:
- **Mutation**:
- **Teardown**:
- **Concurrency / locking**:

无关时删除本节。

## 7. Existing Validation Surface

| Evidence | Path / command / harness | What it currently proves | Gap |
|---|---|---|---|
| | | | |

不要杜撰不存在的测试命令。

## 8. Evidence-derived Invariants

| Invariant | Evidence | Consequence if broken |
|---|---|---|
| | | |

## 9. Constraints

只记录真实的 feature-specific 限制；通用工程规则由 AGENTS.md 负责。

-

## 10. Change Strategy

### Recommended direction

```text
...
```

### Why this fits the existing system
-

### Expected change surface
-

### Explicit non-change surface
-

### Alternatives considered
| Option | Advantage | Cost / risk | Why rejected / deferred |
|---|---|---|---|
| | | | |

## 11. Risks

| Risk | Likelihood / impact | Detection / mitigation |
|---|---|---|
| | | |

## 12. Replan Triggers

-

## 13. Blocking Issues

- None / <issue that must be resolved before delivery>
```

## 使用原则

- 证据优先于篇幅。
- path + symbol 优先于泛泛描述。
- 不知道就写 UNKNOWN。
- Evidence classification 与 workflow state 分离。
- `spec.md` 可以因新证据更新，但不要维护复杂问题生命周期。
- 没有 blocking issue 且 Exit Criteria 满足时即可进入 delivery，不需要额外 READY 状态。
