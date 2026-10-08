---
name: proj-setup
description: 仅在初始化、升级或审查项目工程基准时使用：建立精简 AGENTS、Project Mental Model、Personal Resource Registry，并检查已安装 VibeCoding Core 是否漂移。普通 feature work 不要用。
---

# Proj-Setup V3 — Project Mental Model + Drift-Aware Orchestration Baseline

## Purpose

Proj-Setup 的目标不是“生成若干规范文件”，而是让一个第一次进入项目的 Primary Agent 能立即进入正确工作模式：

1. 理解项目 Mission、Architecture、运行/验证方式与长期不变量；
2. 知道有哪些 Skills / Subagents 可用，以及各自何时触发；
3. 在非平凡任务中保持 **Primary = Orchestrator**，主动委派探索、实现、验证和高风险审查；
4. 只在真正适合的情况下走 `TRIVIAL_FAST_PATH`，避免为一行文档改动制造 Agent 官僚主义；
5. 用独立证据而不是“主模型觉得完成了”作为 Outcome Gate；
6. 建立项目可使用的 **Personal Resource Registry**：明确个人知识库、可复用代码库和其它领域资产源，以及何时优先使用它们；
7. 对 copy-overlay 安装的 Core 做 **Drift Audit**：识别项目仍在使用旧 Skill / AGENTS 规则而中央模板已经升级的情况。

文件是这些认知与行为契约的载体，不是目标本身。

---

## 1. Evidence-first Setup

在写任何基准文件之前先探索事实。优先读取：

- manifests / lock files / build scripts；
- source entry points 与关键 architecture boundaries；
- tests / CI / harness；
- deployment / hardware / runtime config；
- existing `context.md` / `AGENTS.md` / `DESIGN.md`；
- `.codex/config.toml`、自定义 agents 与权限；
- 当前可发现的 Skills，以及它们的触发职责；
- `.agents/VIBECODING_VERSION`（若存在）以及项目安装版本与当前模板/期望版本是否一致；
- 已存在的个人资源登记，例如知识库、共享代码库、组件库、模型库、模板库；
- 当前机器是否能访问这些资源的本地路径或 connector。

### Setup 本身也应证明 Multi-Agent 可用

如果当前环境提供 subagents，Proj-Setup **不得默认由 Primary 单线程扫完整仓库**。应按独立问题委派只读探索，例如：

- Explorer A：实际 build / run / test / lint 命令是什么？
- Explorer B：主要 architecture boundaries、entry points 与 ownership 在哪里？
- Explorer C：现有测试实际证明什么，哪些路径没有自动验证？
- Explorer D：已有 `AGENTS.md` / `.codex` / Skills 是否存在冲突或缺口？

按问题分片，不按目录平均切分。Primary 负责合并 Evidence Packets、解决冲突和决定最终基准。

如果 subagent 能力不可用，允许 Primary 退化执行，但必须在结果中标记 `ORCHESTRATION_UNAVAILABLE`，不能假装已验证多 Agent 工作流。

---

## 2. Baseline Separation

### `context.md` — Project Memory

写入长期有效的：Mission、Strategic Goal、Success Model、Glossary、Architecture Intent、Global Invariants、Major Decisions、Non-Goals、Strategic Unknowns。

### `AGENTS.md` — Slim Project Contract

AGENTS.md 是高频常驻上下文，只保留：

- 最小 orchestration 开关；
- 项目环境与 canonical commands；
- architecture navigation；
- project-specific invariants / conventions；
- resource/repository boundary；
- safety / completion gate。

以下内容不要复制进 AGENTS：

- Skill 的完整 workflow；
- Explorer / Coder / Test Runner / Reviewer 的完整职责；
- model ID / permission 配置；
- Personal Resource 的完整 metadata；
- feature spec/task/design；
- 通用最佳实践说明。

职责分离：

```text
agent config     = WHO
Skills           = HOW
AGENTS.md        = WHAT IS SPECIAL ABOUT THIS PROJECT
```

必须基于 `references/agents-template.md` 生成并按项目事实裁剪。

### Context Budget

根 AGENTS 的默认目标：

- simple project：约 1.5k–2k tokens；
- normal project：约 2k–3k tokens；
- complex project：尽量不超过 4k tokens。

若超过约 4k tokens，Proj-Setup 必须先执行去重审查。

每条常驻内容使用 Admission Test：

> 如果删除这条，未来多数任务是否更容易做出错误的项目级决策？

如果否，则移动到更合适的 Skill / config / context / resource / feature / docs 文件。

### `DESIGN.md` — Experience System

用户可见项目由 `product-design-system` 维护跨功能 UX / visual baseline。`AGENTS.md` 只引用其存在和触发条件，不复制设计细节。

### `.codex/config.toml` — Harness Capability Source of Truth

维护实际可用的 Agent、模型、reasoning effort、权限、并发、MCP 与 orchestration policy。

`AGENTS.md` 负责“何时必须委派”，`.codex/config.toml` 负责“委派给谁、以什么模型和权限运行”。不要在两处重复模型 ID；避免长期漂移。

如果项目希望启用多 Agent，而 `.codex/config.toml` 没有对应能力或自定义 agent 配置缺失，Proj-Setup 必须将其标记为 setup gap，而不是只写一份声称“请使用多 Agent”的 AGENTS.md。

---

## 3. Personal Resource Registry

Proj-Setup 必须区分：

```text
Project Repository
    当前项目事实

Personal Knowledge Sources
    用户长期维护的知识、研究笔记、Source Notes

Reusable Asset Repositories
    用户长期维护的代码、组件、模型、模板与基础设施

External Authoritative Sources
    Web / standards / vendor docs / papers
```

### 3.1 Why this matters

Agent 不应在每个项目里重复学习和重复造轮子。

当用户已有长期个人资产时，默认搜索顺序应倾向：

```text
Project reality
    ↓
Registered personal knowledge / reusable assets
    ↓
Authoritative external evidence when needed
    ↓
New implementation / new research
```

具体顺序可由任务性质调整，但不得跳过已登记的高价值个人资产而直接重复实现。

### 3.2 Resource classes

Registry 至少支持：

- `KNOWLEDGE_BASE`：Obsidian / Markdown / notes / research；
- `CODE_LIBRARY`：通用代码、组件、SDK 封装；
- `MODEL_LIBRARY`：AI model / checkpoint / adapter / evaluation asset；
- `TEMPLATE_LIBRARY`：项目模板、文档模板、UI 模板；
- `DATA_LIBRARY`：数据集、测试向量、样例输入；
- `OTHER`：其它长期个人资产。

不要写死领域名。Embedded、AI、Frontend、Backend 应通过 resource metadata 描述，而不是新增不同格式。

### 3.3 Project-level registry vs machine-local location

长期语义和机器路径必须分开。

#### Versioned project registry

推荐项目根目录维护：

`PROJECT_RESOURCES.md`

记录：

- logical name；
- resource type；
- domains；
- purpose；
- preferred use；
- trust / authority notes；
- whether required or optional；
- local alias/key，而不是强制绝对路径。

这个文件可以提交 Git。

#### Machine-local mapping

机器相关绝对路径推荐维护：

`.agents/resources.local.md`

例如：

```markdown
# Local Resource Paths

- personal-knowledge: D:/Knowledge
- embed-lib: D:/code/embed-lib
- ai-lib: D:/code/ai-lib
```

该文件 SHOULD 被加入 `.gitignore`，除非用户明确希望提交机器路径。

如果用户使用稳定的 sibling relative paths，也可在 versioned registry 中直接记录相对路径，例如：

```text
../embed-lib
../knowledge
```

### 3.4 Setup interaction

只有当前项目第一次建立 Registry，或已有 Registry 失效时，才询问用户。

优先让用户提供：

1. 个人知识库路径或连接方式；
2. 可复用资产仓库路径；
3. 每个资源的逻辑名称与大致领域；
4. 是否允许 Agent 读取；
5. 是否允许 Agent 修改；
6. 是否只作为参考，还是可直接复制/复用资产。

不要每次运行 Proj-Setup 都重复询问已经登记且仍可访问的资源。

如果资源路径可以从当前 workspace / sibling repo 可靠发现，可先提出候选而不是要求用户手输。

### 3.5 Access policy

Registry 必须区分：

- `READ_ONLY`
- `READ_WRITE`
- `COPY_ALLOWED`
- `REFERENCE_ONLY`

默认不要把“可读取”解释为“可修改”。

例如个人知识库通常可以是：

```text
READ_ONLY during normal project work
READ_WRITE only when user explicitly asks to save/update knowledge
```

代码资产库通常可以是：

```text
READ_ONLY as source of reusable assets
COPY_ALLOWED into project when appropriate
READ_WRITE only when user explicitly asks to improve the library itself
```

### 3.6 Resource routing

生成的 `AGENTS.md` 应包含资源路由原则：

- 外部事实未知 → 先查登记的 Personal Knowledge；证据不足再用 `deepnote` / authoritative external source；
- 需要通用实现 → 先查项目已有实现，再查登记的 CODE_LIBRARY，再决定新写；
- 不得把个人知识库内容自动视为当前项目事实；
- 不得因为代码库中存在组件就自动选择它，仍需验证适用性、许可证、版本和验证状态；
- Knowledge Base 是 Evidence Source，不是 Skill；
- Reusable Asset Repository 是 Implementation Source，不是项目依赖声明。

读取 `references/resource-registry-template.md` 生成或更新 Registry。

---

## 4. Automatic Skill Router

Proj-Setup 必须在生成的 `AGENTS.md` 中建立 Skill Router。

核心规则：

> **当任务命中已安装 Skill 的职责时，Primary 应主动使用该 Skill；不要等待用户再次点名。**

Skill Router 必须来自“实际可发现的 Skills”，不得编造不存在的 skill。对于 VibeCoding 常见能力，可按以下职责映射；只有实际可用时才写入项目 AGENTS：

| Trigger | Skill | Purpose |
|---|---|---|
| 项目首次初始化 / 长期基准失真 | `proj-setup` | Project Mental Model + Orchestration Contract |
| 需求含糊、目标/边界/质量标准不清 | `understanding` | Goal Model |
| 陌生领域、关键外部事实或概念风险 | `teach-me` | 证据化领域理解 |
| Logo / App Icon / Functional Icon | `icon-creator` | Product-aware visual symbol design |
| 非平凡代码改动，需要理解现状和策略 | `code-spec` | Evidence + Change Strategy + Goal Board |
| 已有 Goal / Strategy，进入实现与验证 | `spec-workflow` | Adaptive Delivery Loop |
| 深度技术研究 / 文档化论证 | `deepnote` | Evidence-first deep analysis |
| 功能完成，需要 Outcome Audit / 学习抽取 | `archive-and-summary` | Outcome Audit & Learning |
| 需要更新长期项目记忆 | `context-of-user` | Long-term Project Memory |

不要机械串完整 pipeline。只调用当前决策真正需要的最小 Skill 集。

UI/UX 能力已经迁移到独立 Domain Pack；只有实际发现 `design-system` / `ux-design` / `ui-design` / `design-review` 等 Skill 时才写入 Router，不得继续生成已移出的 `product-design-system`。

已有高质量 artifact 时优先复用；不要因为“流程规定”重复跑同一个 Skill。

---

## 5. Delegation & Evidence Value Gate

生成的 `AGENTS.md` 应明确 Primary 的 orchestration 责任，但不要把 worker 变成固定流水线。委派必须同时考虑 information value、decision value 与 critical-path cost。

### Primary 的默认姿态

Primary 是 Goal Controller / Orchestrator。它主要负责：

- WHY / Desired Outcome；
- skill routing；
- risk classification；
- architecture / cross-module decisions；
- evidence synthesis；
- conflict resolution；
- replan；
- final acceptance。

在有合适 worker 时，Primary **不应自行包办**以下高噪声工作：

- 大范围 repository scanning；
- 多条独立 call/data-flow 调查；
- 非平凡生产代码实现；
- verbose build/test/log execution；
- 高风险变更的自我审查。

### 典型路由条件

- 关键代码事实未知且会改变当前决策 → `explorer`
- 非平凡代码实现且边界已足够明确 → `coder`
- meaningful claim 缺少 fresh independent runtime/integration evidence，且结果可能改变 acceptance / replan / next handoff → `test_runner`
- state machine / concurrency / ownership / protocol / security / persistence / destructive compatibility / 大重构中存在 consequential review uncertainty → `reviewer` PRE/POST

额外 worker 工作只有在能增加新证据时才启动；只有其结果可能改变下一 meaningful handoff 时才允许阻塞 critical path。独立检查无真实依赖时应并行，不能因为 worker 已经启动就无条件等待它结束。

### `TRIVIAL_FAST_PATH`

Primary 可以直接完成任务，仅当同时满足：

- 局部、明确、低风险、易回滚；
- 不需要新的 architecture / product / domain 决策；
- 不需要大范围搜索；
- 不改变公共接口、持久化、并发、协议、安全、用户关键流程；
- 验证简单且不会产生大量输出。

如果任一条件不满足，进入 `ORCHESTRATED_PATH`。

---

## 6. Architecture Map

Proj-Setup 应建立导航锚点而不是完整代码百科：

```text
entry points
core domains
platform / adapters
persistence / transport
build / test
external boundaries
```

对每个关键区域说明 Why relevant、owner/lifetime 或重要 invariant；避免目录罗列。

---

## 7. Verification Model

至少确认：

- 最小 build command；
- targeted test 路径；
- full regression 路径；
- integration / hardware / runtime validation（若存在）；
- 哪些行为目前无法自动验证。

不要为了模板要求虚构测试框架或强制安装新工具。测试缺口记录为 gap，由具体目标决定是否补足。

完成声明必须由足够的 objective validation evidence 支持；高风险/自证偏差明显的 claim 在有 decision value 时使用独立 validation。`coder: done` 不是证据。

验证默认按语义完整 implementation batch 集中执行：先明确必需/可选检查与 blocking claim，无依赖项并行；一轮收集独立失败后统一修复；修后只重跑受影响 claim 和必要 integration boundary。

---

## 8. Safety Model

只写项目真实需要的高风险规则，例如：

- history rewrite / force reset；
- push / release；
- destructive migrations；
- 删除关键配置/源文件；
- secrets / credentials；
- hardware destructive operations。

不要复制与项目无关的通用恐吓模板。

---

## 9. Core Drift Audit

VibeCoding 采用 copy-overlay，中央模板升级不会自动更新已有项目。

如果存在 `.agents/VIBECODING_VERSION`：

- 读取 installed core marker；
- 与当前期望/模板版本比较；
- 版本不一致时只审查 Core-managed surfaces：Core Skills、agent configs、AGENTS 中的通用 orchestration 规则；
- 保留项目特有 environment / architecture / safety / validation 事实；
- 删除已经由新版 Core 接管、且与新版策略冲突的旧通用流程，例如“每个 success claim 都必须 Test Runner”；
- 不用整份覆盖的方式破坏项目本地规则。

若 marker 缺失但项目明显来自旧 VibeCoding 模板，标记为 `CORE_VERSION_UNKNOWN` 并做最小 drift audit。

## 10. Orchestration Readiness Gate

### `PROJECT_MODEL_READY`

只有同时满足以下条件才可宣告 READY：

- 新 Agent 能理解 Mission 与 architecture boundaries；
- 已存在的 Personal Resource Registry 可被发现，机器本地映射存在或明确标记 unavailable；
- build/run/test 等事实可追溯；
- `AGENTS.md` 明确 `Primary = Orchestrator`；
- `AGENTS.md` 保留最小 skill-routing / orchestration 开关，不复制 Skill 与 Worker 的完整协议；
- `AGENTS.md` 明确 `TRIVIAL_FAST_PATH`、bounded exploration 和独立 validation/review 的并行原则；
- `AGENTS.md` 通过 Context Budget 检查，默认未无理由超过约 4k tokens；
- `.codex/config.toml` 与实际 agent 能力不冲突；
- 至少完成一次 orchestration smoke check：由独立 Explorer/Reviewer 审查生成的基准，或明确标记 harness 不支持此验证。

### `PROJECT_MODEL_PARTIAL`

可开始低风险工作，但仍存在明确的环境 / architecture / validation / orchestration gap。

### `PROJECT_MODEL_BLOCKED`

缺失事实或 harness 能力会让修改存在显著破坏风险，或者 AGENTS 声称必须委派但实际没有可用 subagent。

---

## 11. Update Policy

Proj-Setup 不要求每个文件都存在或每次都被修改，只修复真实缺失或失真的长期基准。

日常 feature 不应反复触发 proj-setup；长期知识由 `archive-and-summary` 的 Learning Extraction 按需回流。

但如果发现以下情况，应重新运行或局部审查 Proj-Setup：

- Primary 持续单打独斗、忽略已有 subagents；
- Explorer 经常越过当前目标边界，出现 SDK → kernel → bootloader → register 式无证据下钻；
- Test Runner 与 Reviewer 明明独立却长期被串行调度；
- 已安装 Skills 长期不被自动路由；
- agent/model/permission 配置已变化而 AGENTS 仍描述旧行为；
- `.agents/VIBECODING_VERSION` 缺失、落后或与当前 Core 行为不一致；
- build/test/runtime 基线发生长期变化；
- 个人知识库、共享代码库或其它长期资源新增、迁移、重命名或权限发生变化。
