# context.md — Project Memory Template

```markdown
# Project Memory

> Long-term context only. Feature progress, implementation details and archive logs do not belong here.

## 1. Project Mission

- **Problem this project exists to solve**:
- **Primary users / systems**:
- **Long-term value**:

## 2. Current Strategic Goal

跨多个 feature 的当前演进方向：

-

## 3. Success Model

只记录项目级长期质量目标。

| Dimension | Long-term expectation | Why it matters |
|---|---|---|
| | | |

## 4. Glossary

| Term | Definition | Status |
|---|---|---|
| | | CONFIRMED / DRAFT |

## 5. Architecture Intent

代码结构可以变化，但这些长期架构方向应保持，除非重新决策。

| Intent | Rationale | Status / invalidation condition |
|---|---|---|
| | | |

## 6. Global Invariants

| Invariant | Why | Evidence / source |
|---|---|---|
| | | |

## 7. Major Decisions

### <Decision title>

- **Status**: CONFIRMED / CURRENT_DIRECTION / DRAFT / RETIRED
- **Decision**:
- **Rationale**:
- **Alternatives rejected**:
- **Would revisit if**:

## 8. Non-Goals

-

## 9. Strategic Unknowns

| Unknown | Why it matters | Status / owner / validation path |
|---|---|---|
| | | OPEN / ACCEPTED / RESOLVED |

## 10. Memory Changes

仅记录本文件的**长期共识变化**，不记录 feature 完成事件。

| Date | Long-term memory change | Why it changed |
|---|---|---|
| | | |
```

## 维护原则

- 不要求每个章节都有内容。
- 不因为归档 feature 自动追加记录。
- 可从 archive 中提炼长期知识，但 archive 本身不等于长期知识。
- 当旧决策失效，更新状态与 rationale，而不是留下两个同时生效的互相冲突规则。
