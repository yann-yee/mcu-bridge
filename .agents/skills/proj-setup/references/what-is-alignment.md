# 评估标准：什么是一份出色的 Project Orchestration Contract？

V2.1 中，一份优秀的 `AGENTS.md` 不只是“代码库规范”，而是让 Codex 在每个新会话里自动进入正确执行模式的项目级契约。

核心判断：

> **它能否让一个零上下文 Primary 在面对非平凡任务时，主动路由到合适 Skill、委派合适 subagents，并用独立证据完成验收？**

如果答案是否定的，即使 build 命令、编码风格写得很详细，也仍然是不完整的 AGENTS。

---

## 合格度自检

### 1. Orchestrator Identity

- [ ] 明确写出 `Primary = Goal Controller / Orchestrator`。
- [ ] 明确写出 Primary 不是默认 Explorer/Coder/Test Runner/Reviewer。
- [ ] 非平凡任务不应由 Primary 自行完成 discovery → implementation → test → review 全链路。

失败信号：全文只有“你必须如何编码”，没有“你必须何时委派”。

---

### 2. Fast Path / Orchestrated Path

- [ ] 有明确 `TRIVIAL_FAST_PATH`。
- [ ] 有明确 `ORCHESTRATED_PATH`。
- [ ] Fast Path 由风险/未知量/影响范围决定，而不是“改动行数少”。
- [ ] 不把 multi-agent 变成所有请求都固定启动四个 worker 的官僚流程。

---

### 3. Automatic Skill Router

- [ ] 列出当前环境实际可发现的 Skills。
- [ ] 每个 Skill 有任务语义 trigger，而不是“用户输入 `$skill-name` 时”。
- [ ] 明确要求匹配时由 Primary 主动调用，不等待用户再次点名。
- [ ] 已有 artifact 足够时允许跳过 Skill。
- [ ] 不机械串完整 pipeline。

失败信号：Skills 安装了很多，但 AGENTS 中完全没有 routing policy。

---

### 4. Mandatory Delegation Gates

至少必须能回答：

- [ ] 什么情况必须 `explorer`？
- [ ] 什么情况必须 `coder`？
- [ ] 什么情况必须 `test_runner`？
- [ ] 什么情况必须 `reviewer` PRE / POST？

措辞应接近 MUST/SHOULD，而不是“可以考虑”。

推荐最小映射：

```text
unknown code fact affecting decision -> explorer
non-trivial implementation          -> coder
objective success claim              -> test_runner
high-risk strategy/change            -> reviewer
```

---

### 5. Agent Handoff Quality

- [ ] Explorer 收到 bounded question，而不是目录。
- [ ] Coder 收到 Goal Contract，而不是函数级施工脚本。
- [ ] Test Runner 收到 validation claim，而不是只给一条命令。
- [ ] Reviewer 有明确 PRE / POST 目的。
- [ ] workers 返回压缩证据，避免污染 Primary 上下文。

---

### 6. Independent Validation

- [ ] Coder 的“完成/测试通过”不能被当作最终独立证据。
- [ ] completion gate 有 objective validation evidence。
- [ ] 高风险变更有独立 review 条件。
- [ ] 测试不存在时诚实记录 gap，不虚构 harness。

---

### 7. Replan Behavior

- [ ] 明确支持 `REPLAN_REQUIRED`。
- [ ] 新证据可以使 milestone obsolete / split / merge / reorder。
- [ ] 不要求 Agent 忠于过时 checklist。

---

### 8. Harness / Documentation Consistency

- [ ] AGENTS 要求的 agent roles 在实际 `.codex/config.toml`/harness 中可用。
- [ ] permissions 与 role responsibility 匹配。
- [ ] AGENTS 不重复硬编码容易漂移的 model IDs / reasoning effort。
- [ ] 如果 delegation 不可用，明确记录 orchestration gap。

“文档要求 multi-agent，但配置没有 agent”不能判 READY。

---

### 9. Environment Precision

- [ ] install/build/run/test/lint 命令来自真实证据。
- [ ] 工具链、语言、目标平台可追溯。
- [ ] README 与真实配置冲突时有标记，而不是盲抄。
- [ ] 不要求 Agent 猜 npm/yarn、gcc/clang 或 hardware target。

---

### 10. Architecture Navigation

- [ ] 记录少量关键 navigation anchors。
- [ ] 说明 why relevant / ownership / invariant。
- [ ] 不把 AGENTS 写成目录百科。

---

### 11. Project-specific Engineering Invariants

- [ ] 硬规则来自真实项目，而不是项目类型模板。
- [ ] 不默认注入“禁止 malloc”“每个函数必须新增测试”“必须读取 ±15 行”等仪式性规则。
- [ ] 对兼容性、资源、协议、安全、可访问性等长期约束有证据时才写入。

---

### 12. Safety & Git

- [ ] secrets / destructive actions / history rewrite / release 等真实高风险操作有边界。
- [ ] Git checkpoint 与 coherent milestone 对齐。
- [ ] 普通 feature worker 不应顺手修改 `.codex/` 或 agent policy。

---

## Orchestration Smoke Test

若 harness 支持 subagents，Proj-Setup 结束前应让独立 Explorer/Reviewer 阅读生成的 AGENTS 并回答：

> “如果用户现在要求修复一个跨文件、原因未知的 bug，这份 AGENTS 是否会明确迫使 Primary 先路由 Skill / Explorer，再委派实现与验证，而不是主模型自己直接改？”

理想答案必须能指出具体条款。

如果只能回答“看情况”“可以使用 subagent”，说明约束仍然太弱。

---

## 负面清单

- ❌ **Solo Hero Contract**：全文默认所有工作都由一个 Agent 执行。
- ❌ **Skill Blindness**：安装了 Skills，却没有自动路由规则。
- ❌ **Agent Theater**：要求“多 Agent”，但不给 bounded responsibilities。
- ❌ **Directory Sharding**：Explorer 按目录而非问题切分。
- ❌ **Self-Validation**：实现者自己说 pass 就算完成。
- ❌ **Static Pipeline Worship**：所有 feature 都固定跑全部 skills/agents。
- ❌ **Cargo-cult Rules**：把前端/后端/嵌入式通用建议无脑写入任何项目。
- ❌ **Model Duplication**：AGENTS 和 config 同时写具体模型，后续漂移。
- ❌ **Fake Harness**：为了模板完整而编造 tests/build commands。
- ❌ **Checklist Completion Bias**：任务勾完就宣告 goal 完成。

---

## Ready 标准

只有以下结论都成立时，Proj-Setup 才应输出 `PROJECT_MODEL_READY`：

```text
Project facts are grounded.
Architecture anchors are usable.
Validation paths are known or gaps are explicit.
Primary identity is orchestrator-first.
Automatic Skill Router exists.
Mandatory Delegation Gates exist.
Fast Path is bounded.
Harness capabilities match the contract.
Independent validation is required where meaningful.
Replanning is a first-class outcome.
```

否则使用 `PROJECT_MODEL_PARTIAL` 或 `PROJECT_MODEL_BLOCKED`，并明确缺口。
