# Proj-Setup V2.1 — 执行指南

本指南规定 Proj-Setup 如何从“扫描项目并写文档”升级成“建立可执行的 Project Mental Model + Orchestration Contract”。

目标不是让项目看起来配置齐全，而是让下一次新会话中的 Primary Agent 自动进入正确工作模式：先路由 Skill，再根据风险和未知量委派合适的 subagents。

---

## Phase 0 — Harness & Capability Audit

在扫描业务代码之前，先确认 Agent Harness 本身：

1. 根目录是否已有 `AGENTS.md` / `AGENTS.override.md`；
2. 是否存在 `.codex/config.toml`；
3. multi-agent 是否可用；
4. 实际定义了哪些 agent roles、权限和并发上限；
5. 当前环境能发现哪些 Skills；
6. 是否存在项目级 MCP / hooks /特殊权限规则。

不要只检查配置文件“看起来像开启了 multi-agent”。如果当前环境允许，应实际派一个只读 Explorer 完成一项 bounded probe，以证明 delegation path 可用。

若不可用，记录 `ORCHESTRATION_UNAVAILABLE` 或具体 gap。

---

## Phase 1 — Question-sharded Evidence Discovery

Proj-Setup 本身默认采用 orchestrated discovery。

建议并行问题：

### Explorer A — Build / Runtime

回答：
- 实际 install/build/run/debug 命令是什么？
- 工具链、版本、目标平台是什么？
- 哪些命令只是 README 声称但未由配置支持？

### Explorer B — Architecture

回答：
- entry points 在哪里？
- 核心 domain / adapter / platform boundaries 在哪里？
- 哪些状态、ownership、lifetime 或协议边界长期重要？

### Explorer C — Validation

回答：
- targeted test / full regression / integration / hardware validation 分别是什么？
- 现有测试实际证明什么？
- 哪些关键行为没有自动验证？

### Explorer D — Agent Baseline

回答：
- 现有 AGENTS/context/DESIGN 有哪些长期规则？
- 当前 Skills/agents 与文档是否一致？
- 是否存在“配置上支持 multi-agent，但项目规则没有要求 delegation”的断点？

不要把 Explorer 按 `src/`, `tests/`, `drivers/` 机械切目录。

Primary 只接收压缩后的 Evidence Packets；冲突事实再定向追查。

---

## Phase 2 — Low-friction Intent Gap Resolution

只向用户询问无法从项目事实获得、且会改变长期基准的内容，例如：

- 项目 Mission / 当前 strategic direction；
- 真实部署/硬件环境而仓库没有声明；
- 需要人工决定的兼容性、安全、发布策略；
- 团队长期偏好。

不要询问可以由代码、配置、测试或 Explorer 回答的问题。

优先给出已验证事实，再问 1–3 个决策问题，而不是长问卷。

---

## Phase 3 — Build the Project Baselines

按职责更新：

### `context.md`

长期 Project Memory：Mission、Architecture Intent、Global Invariants、Major Decisions、Non-Goals、Strategic Unknowns。

### `DESIGN.md`

仅用户可见产品需要。由 `product-design-system` 建立 Experience System。

### `.codex/config.toml`

确认实际 orchestration capability：
- agent roles；
- model/reasoning 配置；
- permissions；
- concurrency；
- MCP / hooks。

不要在 AGENTS 重复 model ID；配置文件是 capability source of truth。

### `AGENTS.md`

必须基于 `agents-template.md`，并优先生成以下部分：

1. `Operating Mode — Orchestrator First`
2. `Automatic Skill Router`
3. `Mandatory Delegation Gates`
4. `Risk & Cost Routing`
5. `Evidence / Handoff / Replan`
6. 项目事实、命令、architecture navigation、engineering invariants、safety

旧模板中与项目无关的“通用最佳实践”不要自动注入。

---

## Phase 4 — Skill Registry Construction

生成 AGENTS 时，枚举当前可发现的 Skills，并建立 trigger table。

规则：

- Skill 必须真实可用；
- 使用准确名称；
- trigger 写成任务语义，不写“用户输入 `$skill` 时”；
- 明确要求 Primary **主动使用 matching Skill**；
- 已有 artifact 能回答当前问题时，不重复调用；
- 多个 Skill 只选必要组合。

对于 VibeCoding 常见 Skills，可参考：

```text
proj-setup             project baseline / orchestration
understanding          goal ambiguity
teach-me               domain uncertainty
product-design-system  UX / UI / experience intent
icon-creator            logo / app icon / functional icon
code-spec               code evidence / strategy
spec-workflow           implementation delivery loop
deepnote                deep evidence-first research
archive-and-summary     outcome audit / learning
context-of-user         durable project memory
```

只有实际安装的才进入生成文件。

---

## Phase 5 — Delegation Contract Construction

AGENTS 必须把委派从“建议”写成 gates。

至少包含：

```text
unknown codebase fact affecting decision -> explorer
non-trivial implementation              -> coder
objective success claim                  -> test_runner
high-risk strategy/change                -> reviewer PRE/POST
```

同时定义 `TRIVIAL_FAST_PATH`，防止过度委派。

Primary 直接执行非平凡任务必须是 exception，而不是默认。

推荐用以下反模式检查措辞强化：

- Solo Hero
- Skill Bypass
- Agent Theater
- Directory Sharding
- Self-Validation
- Stale Plan Loyalty
- Context Dumping

---

## Phase 6 — Orchestration Smoke Check

在标记 setup 完成之前，验证生成的契约是否真的可驱动行为。

### Static check

确认 AGENTS 明确写有：

- `Primary = Orchestrator`；
- `TRIVIAL_FAST_PATH` / `ORCHESTRATED_PATH`；
- automatic skill routing；
- explorer/coder/test_runner/reviewer gates；
- independent validation；
- replan triggers。

### Capability check

确认 `.codex/config.toml` 与 AGENTS 不矛盾：

- AGENTS 要求的 role 实际存在；
- permission 足够且不过度；
- multi-agent capability 可用；
- model details 不被 AGENTS 重复硬编码。

### Behavioral smoke check

如果 harness 支持 subagent：

- 派一个独立 Explorer/Reviewer 审查生成的 `AGENTS.md`；
- 要求它回答：“面对一个非平凡 bug，Primary 是否会被这份契约明确要求先路由 Skill / Explorer，而不是自己直接改？”
- 若答案含糊，继续强化契约。

这不是让 Reviewer 评价文风，而是验证行为触发是否明确。

---

## Phase 7 — Readiness

### `PROJECT_MODEL_READY`

必须同时具备：

- Project facts 足够；
- commands / validation 可追溯；
- architecture anchors 清晰；
- Skill Router 可执行；
- Mandatory Delegation Gates 清晰；
- harness capability 与文档一致；
- smoke check 通过，或已明确记录无法执行的原因。

### `PROJECT_MODEL_PARTIAL`

低风险工作可进行，但 environment / validation / orchestration 中存在明确 gap。

### `PROJECT_MODEL_BLOCKED`

关键事实或 agent capability 缺失，使项目规则无法安全执行。

---

## Final response

交付时不要只说“已生成 AGENTS.md”。应说明：

- 项目 mental model 的关键事实；
- 建立了哪些 automatic skill triggers；
- 哪些任务现在会强制委派给哪些 roles；
- Fast Path 边界；
- smoke check 结果；
- 仍存在的 orchestration / validation gaps。
