# 环境与代码库发现清单 (V2.1)

在生成项目基准前，先通过文件、配置、代码与 Agent Harness 提取“硬事实”。目标不是扫全仓，而是减少 Primary 猜测、重复提问和单线程上下文污染。

---

## 1. 软件工程底座与语言检测

通过首层配置文件判定主语言与工具链：

- **Node.js**：`package.json` / lockfile / engines / package manager。
- **Python**：`pyproject.toml` / requirements / interpreter constraint / env manager。
- **Rust**：`Cargo.toml` / workspace / toolchain / target。
- **C/C++**：`CMakeLists.txt` / `Makefile` / `Kconfig` / cross toolchain / target hardware。
- **其他**：只记录仓库实际证据，不凭经验补全。

提取：主语言、版本约束、构建系统、包管理器、目标平台。

---

## 2. 构建与运行流程

从真实配置提取：

- install/bootstrap；
- local run/debug；
- targeted build；
- full build/release；
- lint/static analysis；
- deploy/flash（若存在）。

优先信任可执行配置和 CI；README 命令若与配置矛盾，标为 `CONFLICTING` 而不是直接照抄。

---

## 3. 测试与验证模型

寻找：

- test framework/config；
- targeted tests；
- full regression；
- integration/system tests；
- hardware/simulator/manual validation；
- CI checks。

关键不是“有没有 tests/ 目录”，而是回答：

> 这些测试实际证明什么？哪些关键行为仍未被证明？

没有测试框架时记录 gap。不要为了填模板自动安装新框架。

---

## 4. Architecture Navigation

寻找少量长期导航锚点：

- entry points；
- core domains；
- platform/adapters；
- persistence/transport；
- ownership/lifetime boundaries；
- state machines / protocols；
- build/test infrastructure；
- external systems/hardware boundaries。

每个锚点记录“为什么重要”，不要生成目录百科。

---

## 5. Existing Project Instructions

检查：

- `AGENTS.md` / `AGENTS.override.md`；
- `context.md`；
- `DESIGN.md`；
- `.codex/config.toml`；
- `.codex/agents/`、`.agents/` 或其他 custom-agent 配置；
- hooks / MCP / local environment config；
- `.cursorrules`、IDE rules 等历史指令来源。

已有 AGENTS 时采用增量迁移：保留仍真实有效的 project-specific knowledge，删除/降级与项目无关的 cargo-cult 规则。

---

## 6. Orchestration Capability Discovery

这是 V2.1 新增的强制检查。

回答：

1. 当前 harness 是否支持 subagents？
2. multi-agent 是否启用/可用？
3. 实际有哪些 agent roles？
4. 每个 role 的权限是什么？
5. 并发上限是什么？
6. 哪些角色可写、哪些只读？
7. 当前 `AGENTS.md` 是否明确要求 delegation，还是只描述编码规范？
8. 是否存在“配置有 agent，但 Primary 从不调用”的行为断点？

如果可用，执行至少一个 bounded subagent probe 验证能力，而不是只看配置文本。

---

## 7. Skill Discovery

枚举当前环境实际可发现的 Skills。

对每个 Skill 提取：

- exact name；
- trigger / responsibility；
- expected artifact/result；
- 是否适合自动触发；
- 是否与其他 Skill 有职责重叠。

生成 AGENTS 时将其转成 Automatic Skill Router。

规则：

- 不得编造未安装 Skill；
- 不依赖用户每次手动点名；
- 不将所有 Skills 机械串成固定流水线；
- 已有 artifact 足够时优先复用。

---

## 8. Personal Resource Discovery

检查项目是否已经登记：

- `PROJECT_RESOURCES.md`；
- `.agents/resources.local.md`；
- sibling personal repositories；
- 已连接的知识库 / files / apps / plugins；
- 其它长期代码、模型、模板或数据资产。

对每个资源记录：

- logical name；
- type；
- domain；
- purpose；
- local path / connection availability；
- access policy；
- required / optional；
- freshness / version concern。

如果第一次 setup 且没有 registry，将其列入 `USER DECISIONS NEEDED`：

- 个人知识库在哪里 / 如何访问？
- 是否有可复用代码库、模型库、模板库？
- Agent 可以读取还是也可以修改？
- 是否允许复制资产进入当前项目？

不要要求用户为不存在的领域资产创建仓库。

不要把绝对机器路径直接写入 versioned AGENTS，除非用户明确要求。

---

## 9. Domain-specific Risk Discovery

只有项目证据支持时才加入领域硬约束，例如：

- frontend：design system / accessibility / browser support；
- backend：API compatibility / migration / data integrity；
- embedded：ISR、memory/resource limits、hardware access、endianness、timing；
- networking：protocol compatibility、state machine、concurrency、packet ownership；
- security：auth boundary、secret handling、privilege changes。

禁止根据“项目类型标签”自动注入一整套未验证规则。

---

## 10. Suggested Parallel Setup Questions

若 subagents 可用，推荐按问题并行：

```text
Explorer A: What are the verified build/run/toolchain facts?
Explorer B: What are the architecture and ownership boundaries?
Explorer C: What validation paths exist and what do they prove?
Explorer D: What agents/skills/instructions are actually available and where are the orchestration gaps?
```

Primary 负责合并 Evidence Packets，不让每个 Explorer 重复扫全仓。

---

## 11. AGENTS Context Budget Audit

生成或迁移 AGENTS.md 前，按内容来源分类：

```text
KEEP RESIDENT
- project environment / commands
- architecture anchors
- project invariants
- project/resource/safety boundaries
- minimal orchestration switches

MOVE OUT
- generic workflows → Skills
- role contracts → agent TOML
- model/permission details → .codex/config.toml
- durable project intent → context.md
- resource metadata → PROJECT_RESOURCES.md
- feature facts → feature/spec/task artifacts
- long explanations → README/docs
```

对现有 AGENTS 进行去重后再生成新版本。

目标不是保留旧文件每一句，而是保留仍然会改变未来项目决策的高价值事实。

## Output of Discovery

发现阶段最终应形成：

```text
VERIFIED FACTS
CONFLICTING FACTS
UNKNOWN / GAPS
ARCHITECTURE ANCHORS
VALIDATION PATHS
AVAILABLE AGENTS
AVAILABLE SKILLS
REGISTERED PERSONAL RESOURCES
RESOURCE ACCESS GAPS
ORCHESTRATION GAPS
USER DECISIONS NEEDED
```

只有最后一类才优先向用户提问。
