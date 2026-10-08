# Spec-Workflow Delivery Details

仅在根 \`SKILL.md\` 的轻量规则不足时读取。

## Risk Examples

### Low

局部、可逆、行为明确、已有验证路径。

通常 Coder + local check 就足够；没有 meaningful independent claim 时不要自动派 Test Runner。

### Medium

跨文件/接口，或当前实现需要先补一小段证据。

通常 targeted Explorer → Coder；独立验证/review 按 evidence + decision value 决定。

### High

状态机、并发、ownership/lifetime、协议、安全、权限、持久化/迁移、破坏性兼容变更、大型重构。

可能需要：

~~~text
bounded exploration
→ Primary synthesis
→ Reviewer PRE if strategy risk warrants it
→ coherent implementation batch
→ Test Runner / Reviewer POST at meaningful validation boundary
→ Primary acceptance
~~~

这不是固定 pipeline。

## Critical-path Examples

### Blocking before user build / merge

- syntax/target compile failure；
- ABI/public contract break；
- high-confidence source correctness defect；
- missing build identity/provenance required for interpreting next evidence；
- unresolved risk that could invalidate the artifact being handed off。

### Usually non-blocking

- extra mock coverage after required checks pass；
- optional diagnostics；
- future performance tuning；
- repeated deterministic check with unchanged code；
- investigation whose result cannot change current handoff。

Non-blocking work can still be useful; useful does not mean blocking.

## Validation Batch Example

~~~text
Coder:
  change A
  change B
  change C
  cheap local checks
        ↓
freeze coherent batch
        ↓
┌──────────────┬──────────────┬──────────────┐
target/static  host/runtime   Reviewer POST
└──────────────┴──────────────┴──────────────┘
        ↓
collect independent failures
        ↓
one repair batch
        ↓
rerun affected claims
+ necessary integration boundary
~~~

如果某检查失败，不要无理由中止其它独立检查；先收集可以安全获得的失败集合。

## REPLAN_REQUIRED Packet

~~~text
Status: REPLAN_REQUIRED
Observed:
Evidence:
Why current strategy no longer fits:
Suggested options:
Goal impact:
~~~

## Test Runner Assignment

给：

- Validation claim；
- Relevant goal；
- Expected observable behavior；
- Known harness/test hints；
- Constraints；
- Whether this evidence blocks the next handoff。

它验证 claim，不只是执行预写命令。

## Reviewer Modes

### PRE

高风险策略中，检查：

- missing invariants/evidence；
- state/ownership/concurrency；
- protocol/security/persistence；
- 是否存在更小方案；
- validation 是否能发现主要风险。

### POST

检查：

- diff 与 goal alignment；
- regressions/error paths；
- ownership/concurrency；
- security/performance；
- validation gaps。

Runtime evidence 尚未返回时，注明即可；不要阻塞独立静态审查。

## Failure Policy

继续重试的前提是每轮都产生新的、收敛问题空间的 evidence。

升级/停止，当：

- 重试没有新信息；
- 环境/权限阻塞；
- 需要新的用户业务决定；
- 要越过安全/授权边界；
- 高影响未知改变目标/架构；
- 修复开始偏离原 Goal。

## Git Checkpoints

大改动可在语义完整 milestone 后 checkpoint。

Git commit/push/history 操作始终服从项目 AGENTS 和用户授权，不按每个微步骤提交。
