# Engineering Baseline Audit

Audit date: 2026-10-08. Scope: user-invoked proj-setup, documentation/configuration only. Rust source, manifests, workflows and hardware were not modified.

## Readiness

**PROJECT_MODEL_PARTIAL**：Mission、架构导航、Skill Router、资源登记和多代理基准已建立；现存硬件隔离、运行验证和持久 harness 权限仍有缺口。低风险项目内工作可继续，不能据此声称硬件/全量 CI 验收完成。

后续状态：v1.0.2 离线交付已解决下表的默认 probe-rs attach 隔离问题，并建立 tests/json_stdio.rs 离线进程回归、修正 package/lock 和 CI/release 的 locked 构建。此报告保留 setup 时的历史证据；当前实现与验收以 [v1.0.2 验证记录](v1.0.2-validation.md) 和源码为准。真实硬件、跨平台/远程 CI、持久 harness 权限仍未验证。

## Evidence and orchestration

三个独立 Explorer 分别检查 build/test、architecture/ownership、harness/Core drift，Primary 综合证据并保留原用户安全约束。生成后的基准接受独立 Reviewer smoke check，结果见本文后续验证记录。

| Evidence | Conclusion |
|---|---|
| Cargo.toml、Cargo.lock | edition 2024，MSRV 1.95；probe-rs 0.31.0；lib + bin 入口 |
| README.md；src/main.rs、src/lib.rs | 主机侧 Agent 调试中间件；init/flash/clean/debug/doctor |
| src/session.rs；src/buffer/；src/log/ | Session backend Arc/Mutex；历史 Arc/RwLock；采样与接收线程 |
| src/cli/json_session.rs | 动态 schema、JSON 请求/响应/事件；schema_consistency 测试存在 |
| .github/workflows/ci.yml | Rust 1.95.0，fmt/Clippy，三平台 release build；Linux/Windows test，macOS check --tests |
| .github/workflows/release.yml | v* tag 触发三目标产物与集中 GitHub Release；未触发 |
| 本机工具探测 | Rust/Cargo 1.97.1，rustfmt/Clippy driver 可用，cargo clippy -- --help 成功 |

## Core drift

- Installed marker: core=5, generation=gpt6.1。
- 发现的本地中央候选：相邻 vibecoding/README 声明 template/ 为 copy-overlay 源。其 marker 与本项目相同。
- 修改前独立 Explorer 对 .codex（1 文件）、.agents（46 文件）作 SHA-256 比较，全部与该本地 template 一致。没有证据表明本地安装落后；未声称已查询任何远程最新版。
- 项目 AGENTS 缺少当前模板的 orchestrator、routing、bounded exploration、fast path 和资源边界，属于规则漂移；已按实际项目事实迁移。
- proj-setup V3 与 agents-template V4 是不同 artifact 的版本标题，单凭编号不同不能证明版本失配；不擅自改 marker/重写 Core Skills。
- TOML 确认原 developer_instructions 落在 permissions.test-runner.network 表中；官方 OpenAI config schema 将它定义为根字段。本项目已移动到根，并补上 Test Runner target 写权限。
- 官方证据：[OpenAI config schema](https://developers.openai.com/codex/config-schema.json)，此次获取重定向到 [learn.chatgpt.com schema](https://learn.chatgpt.com/docs/config-schema.json)。这是字段/结构核实，不是完整 harness 权限行为测试。
- 中央候选模板有相同 TOML/target 缺陷；本次修正仅在项目，未修改相邻仓库。下次 copy-overlay 可能重新引入；比较时保留项目修正，不整份覆盖。

## Validation model and gaps

| Area | Current evidence / gap | Consequence |
|---|---|---|
| Unit tests | inline tests 覆盖 buffer、serial mock、CLI/JSON/schema、DWARF、日志及 backend 断连错误 | 不证明成功硬件路径 |
| Hardware isolation | v1.0.2 有界调查补充确认 src/probe/probe_rs.rs 的 test_attach_does_not_panic 和 test_breakpoint_id_counter 都调用真实 attach | cargo test/--lib 不是严格无硬件；setup 未运行这些测试 |
| Partial regression workaround | cargo test --lib -- --skip test_attach_does_not_panic --skip test_breakpoint_id_counter 避开已知两处路径 | 只跳过第一个仍会触及硬件；明确报告 skip，不是全量通过或完整隔离保证 |
| Integration | 无 tests/，无已配置 Docker/OpenOCD/MCU 集成 job | 不宣称硬件集成测试通过；未来真实集成须独立 harness |
| OpenOCD | 单测主要是初始/未连接/缺 config；成功 TCL、恢复、退出清理未独立验证 | 生命周期修改需正常退出/Ctrl+C 与协议样本证据 |
| Protocol | schema_consistency 目前仅查命令数、resume 存在、continue 不存在；错误码测试仅部分存在性 | 不能声称完整 schema 与每个实现参数逐项一致 |
| Safety implementation | 旧 AGENTS 有全操作恢复、采样/Semihosting 协调等要求，未做完整 call-flow 审计 | 保留为约束，不升格为已验证实现 |
| Lint debt | buffer/mod.rs、probe/mod.rs、log/mod.rs 等存在 allow；测试还有 wildcard imports | 记录现存冲突，禁止新增，不在 setup 顺带重构 |
| CI dependencies | CI/release 无条件 generate-lockfile，尽管 lock 已跟踪 | 不能声称 CI 精确重放提交的依赖集合；未改 workflow |
| Toolchain | 本机 1.97.1，CI 1.95.0；无 rust-toolchain pin | 本机成功不等于 MSRV/CI 验证 |
| Permissions | 配置 Primary 默认 read-only；本次 runtime 是 unrestricted；Test Runner 禁网依赖预取 | 当前成功探索不证明重启后各角色权限有效；未放宽默认安全配置 |
| Resources | 用户明确选择暂不登记，Registry 为 USER_CONFIRMED UNCONFIGURED | 本机映射 UNAVAILABLE；不自动搜索/复用其他项目，不重复询问 |

## Setup validation record

独立 Test Runner：**PASS**。

| Check | Result |
|---|---|
| cargo fmt --all -- --check | exit 0 |
| git diff --check | exit 0；仅 LF/CRLF 提示 |
| Python tomllib：项目 config + 4 agent role | exit 0；根 developer_instructions、禁网、target write、role 路径断言通过 |
| UTF-8 与文件/Skill 引用 | exit 0；8 个 Skill 和文档存在，无 U+FFFD |
| git check-ignore .agents/resources.local.md | exit 0 |
| git status --short -- Cargo.toml Cargo.lock src .github/workflows | exit 0、无输出；这些路径无改动 |
| 配置语义与本地中央模板比较 | 仅 policy 归属和 target write 两项修正，无模型/其它权限变化 |

独立 Reviewer smoke check：**PASS（修正后）**。首次发现精简遗漏原 CI 合并阻断规则，已补回全部现有 CI job、未来集成 CI 和覆盖率非阻断规则；定向复查通过。硬件/协议/审批约束、架构事实、资源选择、配置结构无其余阻塞。

AGENTS 为约 95 行、5,489 字符。完成去重审查，将按需 API/借用/TCL/CI 经验迁到工程笔记，未复制完整 Skill/worker 协议。未安装 tokenizer，精确 token 数 UNKNOWN；独立审查判断规模符合约 4k 预算，无明显超预算风险。

未运行 Rust build/test/Clippy 回归，也未接触硬件、触发 CI、commit/push 或修改相邻中央仓库。当前 unrestricted harness 的探索/审查成功不证明重启后受限角色可编译；持久权限及硬件缺口仍使 readiness 保持 PROJECT_MODEL_PARTIAL。
