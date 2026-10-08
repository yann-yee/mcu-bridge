# AGENTS.md — mcu-bridge Project Contract

修改或构建前通读本文件。长期意图见 `context.md`；按需工程细节见 `docs/engineering-notes.md`。项目契约服从系统/运行环境约束及用户明确指示。

## 1. Operating Contract

- **Primary = Orchestrator**：负责目标、跨模块决策、证据整合、重规划与验收。Agent 能力/权限以实际 harness 和 `.codex/config.toml` 为准，不在此复制模型配置。
- **TRIVIAL_FAST_PATH** 仅限局部明确、低风险、易回滚、无需新架构/领域决策或广泛搜索、验证简单，且不改变公共接口、持久化、并发、协议、安全或关键用户流程。否则走 **ORCHESTRATED_PATH**，按未决问题委派有信息价值的探索/实现/验证/审查，不固定流水线。
- 探索必须 bounded：列出当前决策所需的问题、范围和停止条件。独立验证/审查无真实依赖时并行；只有证据可能改变下一交接/验收时才阻塞。复用未变化代码的有效证据。
- 最小一致改动后集中验证。修改前阅读受影响位置上下至少 15 行及必要调用方，理解所有权和副作用；证据推翻策略时重规划。
- 命中已安装 Skill 职责时主动使用，不等再次点名。只调用当前决策需要的最小集合，复用已有有效 artifact：

| 当前需要 | Skill（`.agents/skills/<name>/SKILL.md`） |
|---|---|
| 初始化/审查长期基准 | `proj-setup` |
| 意图或高影响边界不清 | `understanding` |
| 陌生领域会影响用户决策 | `teach-me` |
| 非平凡代码的现状/实现路径未明 | `code-spec` |
| 已有 Goal/spec/task 的多步交付 | `spec-workflow` |
| 外部可核验证据/研究笔记 | `deepnote` |
| 交付后的 Outcome Audit/归档 | `archive-and-summary` |
| 更新长期项目记忆 | `context-of-user` |

其他环境 Skills 仅在职责匹配时使用，不虚构未安装能力。

## 2. Project Environment & Commands

主机侧 Rust 工具，edition 2024、MSRV 1.95；CI 固定 1.95.0。本机 Windows/PowerShell，也支持 Git Bash；2026-10-08 核实本机 Rust/Cargo 1.97.1、Clippy driver 可用。目标 Windows/Linux；CI 另有 macOS 构建。`Cargo.lock` 必须跟踪，当前 probe-rs 锁定 0.31.0。

| 用途 | Canonical command |
|---|---|
| 首次依赖准备（先于编译） | `cargo fetch` |
| 最小编译 | `cargo check` |
| 完整构建 | `cargo build --release` |
| 格式检查 / 修正 | `cargo fmt --all -- --check` / `cargo fmt --all` |
| 新环境检查 Clippy driver / lint | `cargo clippy -- --help` / `cargo clippy --all-targets --all-features -- -D warnings` |
| 定向测试 / 单元测试 / 全量回归 | `cargo test <filter>` / `cargo test --lib` / `cargo test` |
| JSON schema 回归 | `cargo test schema_consistency` |
| 本地文档 | `cargo doc --no-deps --open` |
| 无硬件 CLI 入口 | `cargo run -- --help` |

硬件运行参数见 `README.md` 和 `src/cli/mod.rs`，执行前核对目标板与 ELF；不要为验证 setup 接触硬件。
Linux 构建需要 libudev/libusb 开发库，以 CI 安装步骤为准。OpenOCD 路径属于机器配置，不硬编码历史用户路径。

## 3. Architecture Navigation

| 区域 | 导航 | 责任/边界 |
|---|---|---|
| 入口与交互 | `src/main.rs`、`src/cli/mod.rs`、`src/cli/debug.rs` | Clap 分发；debug 编排 REPL/JSON、会话、采样、日志 |
| 协议 | `src/cli/json_session.rs`、`src/error.rs` | 动态 schema、请求/响应/事件、稳定错误码 |
| 配置与共享操作 | `src/config.rs`、`src/backend.rs`、`src/operations.rs` | 配置/后端选择；flash/doctor 共享业务边界 |
| 探针与会话 | `src/probe/mod.rs`、`src/probe/{probe_rs,openocd}.rs`、`src/session.rs` | DebugProbe 隔离驱动；Session 持有共享 backend mutex |
| 历史与接收线程 | `src/buffer/mod.rs`、`src/buffer/serial.rs` | Sampler/SerialMonitor 管理线程；历史通过 Arc/RwLock 共享 |
| 日志与符号 | `src/log/`、`src/dwarf/` | LogChannel 的 RTT/UART/Semihosting；独立 ELF/DWARF 解析 |
| 验证与发布 | 各模块内测试、`tests/json_stdio.rs`、`.github/workflows/{ci,release}.yml` | tests/ 为离线进程回归，尚无真实硬件 harness；release 由 v* tag 触发 |

高风险边界：共享 SWD 访问与线程退出、探针恢复、OpenOCD 子进程/TCL、Flash、JSON 状态机。

## 4. Project Invariants & Conventions

以下是必须遵守的约束，不代表每条已由现有实现/测试证明；已知缺口见 `docs/engineering-baseline.md`。

- 第三方 API 不凭记忆编造。probe-rs 调用前检查锁定版本及相关源码/文档签名，侦察限于所需 API，再用编译器核实；借用/TCL 注意事项见工程笔记。
- 禁止通过新增 `allow(clippy::…)`、`allow(dead_code/unused)` 或 unsafe 掩盖问题，除非用户明确批准且有理由。不得用 unsafe 实现同步；现存 suppression 是债务，不是新增许可。
- 改 DebugProbe/LogChannel 签名前列出全部实际 implementor/caller（含 mock），同步更新并验证：ProbeRsBackend/OpenOcdBackend、RttChannel/UartChannel/SemihostingChannel 等。
- Flash 写入（含 Flash 断点）默认回读校验，只允许用户明确 `--no-verify` 关闭。`--chip` 保留用户原值，模板使用 probe-rs 精确 target 名。
- 每次硬件操作检查 `is_connected()`，失联走 `try_recover()`，恢复期间历史标记 `gap=true`。SWD 操作通过共享 mutex 串行化；Semihosting 处理时采样必须等待。
- OpenOCD 启动等 TCL 端口就绪，默认 localhost:6666；命令默认 5s 超时后 kill/restart/reattach；退出/panic 通过 Drop/guard 清理子进程。修改生命周期须验证正常退出和 Ctrl+C，无残留。
- 缓冲读写使用明确、最小锁临界区；接收/采样线程退出需清理资源。同步只用标准库 Mutex/RwLock/Arc/Barrier/Condvar。串口兼容 COM3 和 /dev/ttyACM0；缓存用 `dirs::home_dir()/.mcu_bridge/`。
- JSON stdout 每行一个完整对象，以 `serde_json::to_string` + 换行输出。动态 schema 是协议事实来源；改参数/响应时同步 schema 并测试实际一致性。
- 既有 12 错误码名称/语义不可改，只能追加：`E_STATE`、`E_PARAM`、`E_BACKEND`、`E_PROBE`、`E_PROBE_LOST`、`E_FLASH`、`E_NO_DWARF`、`E_NO_SEMIHOSTING`、`E_FLASH_BP_DISABLED`、`E_FLASH_BP_LIMIT`、`E_SERIAL`、`E_INTERNAL`。
- 新增函数/模块/API 编写并运行对应测试，bug 修复须有相关回归证据。硬件单元测试用 trait mock，禁止连接真实硬件；真实 MCU/OpenOCD 集成放 tests/ 单列验证。
- 模块 snake_case；错误用 thiserror 与可读消息。所有 pub fn/trait/struct 有用途、参数、panic/error 的 `///` 文档；导入按 std → 第三方 → crate 分组，不新增 `use super::*`/`use crate::*`。

## 5. Personal Resources

登记：`PROJECT_RESOURCES.md`；机器路径：`.agents/resources.local.md`（gitignored）。先查项目事实，再查相关已登记知识/代码资产，证据不足再查权威外部资料/用 deepnote，最后决定新研究/实现。
知识库是证据源，代码库是实现来源；不是 Skill、当前项目事实或自动依赖。复用前检查适用性、许可、版本和验证状态；读取、复制、修改权限分开，未登记不视为已授权。

## 6. Validation & Completion

代码变更交付前必需：fmt、Clippy 零 warning、release build、cargo test；协议变更另跑 schema_consistency，trait 变更验证全部实现，OpenOCD 生命周期变更另验退出路径。文档/基准变更验证引用、配置解析、规则保留和独立 smoke review，不声称 Rust 回归已通过。
v1.0.2 已移除 probe-rs 单测中的真实 attach 路径；默认 cargo test 使用离线 mock。进程测试的 ignored fixture 由父用例显式执行，不是跳过验收。不得重新引入默认硬件访问；真实设备测试须独立、显式 opt-in。
Clippy 不可用时报告环境缺口，fmt/check 与全 src 同类 lint 搜索只作替代预检，不能声称 Clippy/CI 已通过，也不能用 allow 绕过。
完成声明须有目标和不变量的客观证据；worker “done”不等于验收。阻塞证据与非阻塞诊断分开，无依赖检查并行；无法验证的高风险行为明确列出。
CI 当前 Linux/Windows 跑 cargo test，macOS 仅 cargo check --tests；这不是三平台运行测试全部通过的证明。
合并必须通过现有 fmt、Clippy、三平台 build-and-test 全部 CI job，任一失败阻断；硬件集成 CI 启用后同样阻断。覆盖率当前不作阻断条件。

## 7. Safety & Long-term Artifacts

- git commit、push、publish、tag/release 或触发 CI 前展示完整变更并获得用户明确批准；集中少数逻辑提交。不得擅自改写历史/force-push。
- 不泄露/提交凭据，不撤销用户已有变更。普通 feature 不顺带修改 `.codex/`、`.agents/`；明确 setup 属于允许的配置工作。
- 按需读 context.md、资源登记、工程笔记和当前 feature Goal/spec/task；历史诊断不是当前计划。
- docs/engineering-baseline.md 保存 setup 证据/缺口，不是永久流程或功能待办。更新长期共识放最小正确 artifact，不重复制造文档。
