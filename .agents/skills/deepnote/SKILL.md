---
name: deepnote
description: 当当前决策需要外部可核验证据，或用户要求来源笔记/主题研究/知识笔记时使用。负责证据，不替调用方做最终工程或产品决策。
---

# DeepNote V2 — Evidence Engine

## Purpose

DeepNote 不再只负责“读论文并写笔记”。

它是 VibeCoding 的通用外部证据入口，负责：

- source discovery；
- authority assessment；
- evidence extraction；
- claim verification；
- conflict detection；
- applicability analysis；
- evidence gap tracking；
- research-note / knowledge-note synthesis。

DeepNote 不拥有最终产品、架构或工程决策。

调用方 Skill 负责 WHY / Decision；DeepNote 负责 Evidence。

核心原则：

> **Evidence before Decision. Source authority, claim status and applicability must remain distinct.**

---

## When to Use

使用 DeepNote 当：

- 用户要求高保真论文 / 技术文档 / 标准笔记；
- 当前工程决策依赖不确定的外部事实；
- Agent 在编码 / 调试 / 架构前需要权威资料确认；
- 一个 claim 需要多个来源交叉验证；
- 不同来源存在冲突；
- 用户要求把研究整理为长期个人知识库笔记；
- `teach-me` 需要外部领域证据。

不要仅因为“想多了解一些”就机械运行 DeepNote。研究必须服务于明确问题、来源或知识沉淀目标。

---

## Four Modes

### 1. SOURCE_NOTE

目标：高保真记录一个主要 Source。

适用于：

- academic paper；
- datasheet；
- reference manual；
- specification / RFC；
- application note；
- technical report；
- court opinion / regulation；
- authoritative documentation。

回答：

> 这份 Source 实际说了什么？

如果 Source 是学术论文，继续使用现有：

- `references/evidence-checklist.md`
- `references/plan-template.md`
- `references/writing-guidelines.md`
- `references/what-is-depth.md`

这些文件现在被视为 **Academic Paper SOURCE_NOTE 专用规则**，不应机械套到其它类型 Source。

---

### 2. EVIDENCE_BRIEF

目标：快速回答当前决策前必须确认的一个外部事实。

典型问题：

- LVGL 是否 thread-safe？
- STM32H7 DMA + D-Cache 在什么条件下需要 cache maintenance？
- FreeRTOS 哪些 API 可在 ISR 调用？
- 某协议的 timeout / timing 规则是什么？

输出应短、证据密度高，不追求百科完整。

读取：

- `references/common/evidence-principles.md`
- `references/common/source-authority.md`
- `references/evidence-brief-template.md`

DeepNote 只返回 Evidence Brief；调用方 Skill 决定工程含义。

---

### 3. TOPIC_RESEARCH

目标：围绕一个问题进行多来源研究并形成可复用 Research Note。

例如：

- 嵌入式测试有哪些成熟方法？
- HardFault 调试的通用方法论是什么？
- AI 训练中有哪些已被复现的显存优化方法？

流程：

```text
Research Question
    ↓
Source Discovery
    ↓
Authority Classification
    ↓
Evidence Extraction
    ↓
Claim Matrix
    ↓
Conflict / Applicability Analysis
    ↓
Synthesis
```

读取：

- `references/common/evidence-principles.md`
- `references/common/source-authority.md`
- `references/topic-research-template.md`

---

### 4. KNOWLEDGE_NOTE

目标：把已经核验过的证据压缩成可长期维护的个人知识库笔记。

回答：

> 关于这个 Concept / Pattern / Rule，我目前可靠知道什么？

它不是 Source 摘要。

例如：

- `NXP UM10204.md` 是 SOURCE_NOTE；
- `I2C Bus Recovery.md` 是 KNOWLEDGE_NOTE。

读取：

- `references/common/evidence-principles.md`
- `references/common/source-authority.md`
- `references/knowledge-note-template.md`

Knowledge Note 应适合 Obsidian 链接与后续增量维护。

---

## Evidence Model

每个重要 Claim 必须尽量分别记录三个维度。

### Source Authority

- `PRIMARY_OFFICIAL`
- `VENDOR_OFFICIAL`
- `REFERENCE_IMPLEMENTATION`
- `SECONDARY_TECHNICAL`
- `COMMUNITY`
- `PERSONAL_ANALYSIS`

### Claim Status

- `VERIFIED`：有直接高质量证据，或多个独立来源一致支持；
- `SUPPORTED`：证据支持但仍存在条件、间接性或复现不足；
- `CONFLICTING`：高价值来源存在实质冲突；
- `UNRESOLVED`：当前证据不足；
- `REPORTED`：仅记录来源声称，尚未验证。

### Applicability

- `DIRECT`：与当前版本、硬件、法律辖区、数据集或环境直接匹配；
- `CONDITIONAL`：只在明确条件下适用；
- `NOT_APPLICABLE`；
- `UNKNOWN`。

禁止把：

```text
来源很权威
```

自动等同于：

```text
一定适用于当前项目。
```

---

## Evidence Principles

所有 Mode 都遵循：

1. 优先 Primary / Official / Authoritative Source。
2. 事实、来源声称、推断、个人分析必须分开。
3. 关键数字、版本、条件、限制必须可回溯。
4. 多来源研究必须显式处理冲突，不允许平均掉差异。
5. 缺失信息使用 `ⓘ` 明示，不得补全想象。
6. 对当前决策没有影响的信息不要无限扩展。
7. Source 的发布时间、版本、适用硬件/软件/地区必须在相关时检查。
8. 一个实现示例不能自动升级为规范要求。
9. 一个论文作者报告的收益不能自动升级为独立复现。
10. 一个厂商建议不能自动升级为跨平台通用规则。

详细规则读取：

- `references/common/evidence-principles.md`

---

## Source Discovery Policy

DeepNote 可以主动搜索外部资料，而不要求用户先提供全文。

根据领域优先使用相应 Source Hierarchy。

示例：

### Embedded

```text
Specification / Architecture Manual
Reference Manual / Datasheet / Errata
Official Application Note / SDK / Driver
Reference Implementation
Vendor Forum / Official Example
Community
```

### AI

```text
Paper
Official Code
Independent Reproduction
Benchmark / Engineering Report
Community
```

### Legal

```text
Statute / Regulation
Court Opinion
Official Guidance
Authoritative Repository
Secondary Commentary
```

DeepNote 提供通用权威性框架；领域 Skill 或当前任务负责具体 Source 优先级。

---

## Workflow

DeepNote 的认知阶段仍保持三段式，但按 Mode 自适应，不再强制所有任务套论文 checklist。

### Phase 1 — Evidence Collection

建立 Evidence Ledger：

- Research Question / Source；
- Claim；
- Evidence；
- Source Authority；
- Location / URL / Section；
- Conditions；
- Claim Status；
- Applicability；
- Conflict / Gap。

### Phase 2 — Evidence-bound Plan

根据 Mode 决定输出结构。

- SOURCE_NOTE：按 Source 自身结构和重要机制组织；
- EVIDENCE_BRIEF：围绕单个问题和 implication；
- TOPIC_RESEARCH：按问题、候选观点、证据和冲突组织；
- KNOWLEDGE_NOTE：按 concept、mechanism、engineering implications、sources 组织。

计划中的每个关键结论必须绑定 Evidence。

### Phase 3 — Synthesis

只根据已收集 Evidence 写作。

新洞察可以出现，但必须标记为：

- `[分析]`
- `[推测]`

并说明依据。

---

## EVIDENCE_BRIEF Contract

Evidence Brief 应尽量让调用方 Skill 可以直接消费：

```text
Question
Finding
Claim Status
Applicability
Conditions
Evidence
Conflicts
Unknowns
Engineering / Decision Implication
Sources
```

DeepNote 不把 `Engineering Implication` 自动变成项目决定。

例如：

```text
Evidence says API is not generally thread-safe.
```

不自动等于：

```text
项目必须新增 mutex。
```

调用方还需结合 architecture / execution model 决定。

---

## KNOWLEDGE_NOTE Contract

个人知识库模式建议使用轻量 Frontmatter：

```yaml
---
type: concept
domain: embedded
status: verified
source_level: authoritative
---
```

推荐 status：

- `draft`
- `reviewed`
- `verified`
- `deprecated`

推荐 source_level：

- `authoritative`
- `vendor`
- `reference-implementation`
- `secondary`
- `community`
- `personal-analysis`

不要把 Frontmatter 变成数据库工程；只保留会帮助未来检索和判断可信度的字段。

---

## Interaction with Teach-Me

`teach-me` 是用户学习与决策 Owner。

DeepNote 是外部 Evidence Provider。

```text
Teach-Me
    ↓ Research Questions
DeepNote EVIDENCE_BRIEF / TOPIC_RESEARCH
    ↓ Evidence Packet
Teach-Me
    ↓ Domain Model + Options + Decision Contract
User
    ↓ confirmed understanding
DeepNote KNOWLEDGE_NOTE (optional)
    ↓
Personal Knowledge Base
```

如果只是 Agent 执行前需要事实核验，DeepNote 可单独运行，不需要 Teach-Me。

---

## Interaction with Other Skills

典型：

```text
code-spec / hardware-debug / embedded-testing / reviewer
      ↓ external fact unknown
deepnote EVIDENCE_BRIEF
      ↓ evidence
caller skill continues decision
```

DeepNote 不修改 caller 的 Goal、Architecture Intent 或 Decision Contract。

---

## Exit Criteria

### SOURCE_NOTE Ready

- Source 的关键机制、证据和限制可追溯；
- 不把摘要当全文；
- 缺失项显式标记；
- 学术论文仍满足原有 high-fidelity 要求。

### EVIDENCE_BRIEF Ready

- Question 明确；
- Finding 有证据；
- Authority / Status / Applicability 已区分；
- 条件与冲突没有被隐藏；
- 调用方知道什么已知、什么仍未知。

### TOPIC_RESEARCH Ready

- 关键观点有多个可信来源或明确单源限制；
- 主要冲突已显示；
- 适用条件已分析；
- 结论没有超出 Evidence。

### KNOWLEDGE_NOTE Ready

- 内容是 distilled knowledge，不是 Source 摘要；
- 核心 Claim 有来源锚点；
- status / source_level 合理；
- 适合未来增量更新；
- 不确定项没有伪装为 verified knowledge。