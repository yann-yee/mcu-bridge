# Feature Intent / Goal Model Template

用于 `user_plan/<feature>/<feature>.md`。

模板不是表单；无关章节可以删除。目标是让新 Agent 能准确理解“为什么做、要改变什么、什么叫做好”，而不是提前预测代码实现。

```markdown
# <Feature> — Intent & Goal Model

## 1. Project Alignment

- **Project goal**:
- **Current strategic goal**:
- **This feature contributes by**:

## 2. Observed Problem

- **Current state**:
- **Who / what is affected**:
- **Evidence**:
- **Why this matters now**:

## 3. Desired Outcome

完成后应出现的系统/用户结果：

-

## 4. Solution Direction

仅记录已经确认的策略级方向；不要写函数、类、文件级施工步骤。

-

## 5. What Good Looks Like

只保留相关质量维度。

### Functional
-

### Architecture
-

### Maintainability / Simplicity
-

### Performance / Resource Use
-

### Reliability / Recovery
-

### Compatibility
-

### Security / Privacy
-

### Observability
-

### UX / Accessibility
-

## 6. Invariants & Constraints

| Constraint / Invariant | Why it exists | Source / evidence |
|---|---|---|
| | | |

## 7. Scope

### Essential
-

### Supporting
-

### Later
-

### Non-Goals
-

## 8. Critical Scenarios

只记录真正影响目标或验收的关键场景，不穷举所有理论边界。

| Scenario | Expected outcome | Why important |
|---|---|---|
| Happy path | | |
| Important failure/recovery | | |

## 9. Unknowns & Assumptions

| Item | Risk | Status | Validation / owner |
|---|---|---|---|
| | HIGH / MEDIUM / LOW | UNRESOLVED / ASSUMED / ACCEPTED | |

## 10. Acceptance Evidence

说明什么证据能够证明目标已经实现。这里定义“要证明什么”，不要提前绑定不存在的测试命令。

- **Claim A**:
  - Evidence required:
- **Claim B**:
  - Evidence required:

## 11. Readiness

`GOAL_READY` / `NOT_GOAL_READY`

### Remaining assumptions
-
```
