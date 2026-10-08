# AGENTS.md V4 — GPT-6 Slim Project Contract

本模板用于生成项目根目录的 \`AGENTS.md\`。

\`AGENTS.md\` 是高频常驻上下文。只保留**当前项目特有、且未来多数任务都可能影响正确决策**的信息。

~~~text
agent config = WHO / permissions
Skills      = HOW / domain methods
AGENTS.md   = WHAT IS SPECIAL ABOUT THIS PROJECT
~~~

不要在 AGENTS 中复制完整 Skill workflow、Agent 职责、模型 ID、feature spec/task 或通用工程教程。

## Context Budget

默认目标：

- simple project：约 1.5k–2k tokens；
- normal project：约 2k–3k；
- complex project：尽量 <= 4k。

Admission Test：

> 删除这条后，未来多数任务是否更容易做出错误的项目级决策？

如果否，移动到 Skill、agent config、context.md、PROJECT_RESOURCES.md、DESIGN.md、feature artifact 或普通 docs。

---

# 推荐生成结构

~~~markdown
# AGENTS.md — Project Contract

## 1. Operating Contract

- Primary owns the goal, cross-cutting decisions, replanning and final acceptance.
- Use a Skill or worker only when it owns the current unresolved decision or adds useful evidence; topic similarity alone is not enough.
- Extra exploration, validation or review must add new evidence; it may block the next handoff only when that evidence can change the handoff decision.
- Prefer coherent implementation batches, then concentrated validation. Do not default to “edit a little → test a little → repeat”.
- Independent checks with no real dependency should run in parallel. Reuse fresh evidence for unchanged code and rerun only affected claims plus necessary integration boundaries.
- If a worker result no longer blocks a decision, advance useful work or stop/defer that worker instead of idly waiting.
- Read only project context needed for the current task. Do not perform a full repository/doc tour by default.
- Prefer the smallest coherent change that achieves the outcome.
- If evidence invalidates the current strategy, replan rather than forcing a stale plan.
- Explicit durable user corrections (“以后… / 不要再… / 每次都…”) and repeated costly failures should be persisted to the smallest correct project artifact instead of waiting for another recurrence.

## 2. Project Environment

> Include verified facts only.

- **Target / runtime:** {verified}
- **Languages / versions:** {verified}
- **Toolchain / package manager:** {verified}
- **Primary development environment:** {verified}

### Canonical commands

| Purpose | Command |
|---|---|
| Bootstrap | {verified or N/A} |
| Targeted build | {verified or N/A} |
| Full build | {verified or N/A} |
| Targeted test | {verified or N/A} |
| Full regression | {verified or N/A} |
| Run / debug / flash | {verified or N/A} |
| Lint / static analysis | {verified or N/A} |

Missing validation paths are gaps; do not invent commands.

## 3. Architecture Navigation

| Area | Path / Symbol | Responsibility | Boundary / Invariant |
|---|---|---|---|
| Entry | {...} | {...} | {...} |
| Core | {...} | {...} | {...} |
| Platform / Adapter | {...} | {...} | {...} |
| Transport / Persistence | {...} | {...} | {...} |
| Build / Test | {...} | {...} | {...} |

### High-risk areas

- \`{path::symbol}\` — {why it is risky}

Do not turn this into a repository tree.

## 4. Project Invariants & Conventions

Only durable, project-specific rules supported by evidence or explicit user decisions.

Examples:

- {public API / ABI compatibility}
- {ownership / lifetime invariant}
- {memory / timing / concurrency constraint}
- {persistence / protocol compatibility}
- {component reuse rule}
- {error / observability requirement}
- {known repeated operational pitfall and its preventive rule}

## 5. Personal Resources

- Registry: \`PROJECT_RESOURCES.md\`
- Local path map: \`.agents/resources.local.md\`

Use registered knowledge/code assets when relevant, but treat them as evidence or reusable sources, not automatic project truth/dependencies.

## 6. Validation & Completion

Before a validation batch, distinguish:

- **blocking evidence** — can change whether the next build/merge/release/acceptance handoff is allowed;
- **non-blocking evidence** — useful diagnostics, extra coverage or future optimization that may run in parallel or be deferred.

Primary accepts the outcome when:

- Desired Outcome has objective evidence;
- relevant project invariants still hold;
- consequential claims have sufficient validation;
- independent review/validation is used when it can materially improve confidence;
- residual high-risk unknowns are resolved or explicitly accepted.

A worker saying “done”, a green checklist, or a large number of tests is not sufficient by itself.

## 7. Safety & Repository Boundaries

Include only real project restrictions, for example:

- {generated/vendor paths that must not be edited}
- {deployment/data-loss/destructive-operation boundaries}
- {credential/secret restrictions}

Defaults:

- no force-push / history rewrite without explicit authorization;
- no secret disclosure or commit;
- do not modify \`.codex/\` / \`.agents/\` as a side effect of ordinary feature work, except when the user explicitly requests workflow/configuration changes.

## 8. Long-term Artifacts

Read only when relevant:

- \`context.md\` — durable project intent and invariants;
- \`DESIGN.md\` — cross-feature UX baseline;
- \`PROJECT_RESOURCES.md\` — registered personal resources;
- feature Goal/spec/task/design — active truth for current work;
- historical diagnostics / raw validation — evidence referenced by active truth, not competing current plans.

Do not duplicate their detailed contents here.
~~~

## Generation Rules

1. Project-specific over generic.
2. Reference instead of copying.
3. Omit empty sections.
4. Keep rules concise; GPT-6 should infer routine steps.
5. Do not duplicate Skill descriptions or agent/model configuration.
6. Do not require reading docs unless the current task needs them.
7. Do not mandate Test Runner/Reviewer for every change.
8. Do not encode a fixed worker pipeline in AGENTS.
9. Persist repeated operational failures only when a concrete preventive rule exists.
10. If generated AGENTS exceeds ~4k tokens, remove duplicated workflow prose before adding more.

## Setup Acceptance

AGENTS is ready when a new Primary can quickly answer:

- What is special about this project?
- How is it built and validated?
- Where are the important architecture boundaries?
- Which invariants and repository/resource boundaries matter?
- What repeated project-specific mistakes should not happen again?
- Which evidence is truly blocking versus merely useful?

If those are clear, additional generic workflow detail belongs elsewhere.
