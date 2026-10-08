# Project Memory

只保存跨功能长期认知。工程命令/行为契约见 AGENTS.md；一次性验证与缺口见 docs/engineering-baseline.md。

## 1. Project Mission

**CONFIRMED**（README.md、Cargo.toml、用户工程契约）：mcu-bridge 是面向 AI Agent 的主机侧嵌入式调试中间件。用缓冲历史解耦 Agent 秒到分钟级思考与 MCU 微秒级执行，把调试和日志统一到 JSON-Lines 入口。人类通过 CLI/REPL 使用同一核心能力。

## 2. Current Strategic Goal

**CURRENT_DIRECTION**：在跨平台主机上提供可自动化的调试闭环，同时保持真实硬件访问、历史数据和协议状态的可靠性。具体 feature 优先级由当前用户目标决定，不从历史 P0/P1/P2 注释推定开发计划。

## 3. Success Model

| 维度 | 长期期望 | 理由 |
|---|---|---|
| 安全与可靠性 | Flash 默认校验，断连可恢复，采样缺失显式可见，子进程可清理 | Agent 需要可信观测与可诊断失败 |
| 自动化兼容 | JSON 每行一对象，动态 schema，错误码语义稳定 | 机器调用方能够发现接口并恢复错误 |
| 可验证性 | 核心业务可注入 trait mock，真实硬件验证与单元测试分离 | 无硬件环境也能回归，不意外操作设备 |
| 可移植性 | Windows/Linux 支持，平台路径不硬编码 | 主机侧工具不绑定单台开发机 |

以上是质量要求，不代表所有硬件保证已验证。

## 4. Glossary

| 术语 | 含义 | 状态 |
|---|---|---|
| DebugProbe | 调试探针后端统一边界，当前含 probe-rs/OpenOCD | CONFIRMED |
| LogChannel | RTT/UART/Semihosting 日志通道边界 | CONFIRMED |
| DebugBuffer / LogBuffer | 供慢速调用方查询的采样/日志历史 | CONFIRMED |
| gap | 无法取得有效采样时的显式历史标记 | CONFIRMED |
| Halted / Running / Recovering | 会话暂停、运行、恢复语义；Recovering 的完整联动仍需验证 | CONFIRMED（语义） |

## 5. Architecture Intent

| 意图 | 理由 | 状态 / 重审条件 |
|---|---|---|
| CLI/协议与探针驱动通过 DebugProbe 分离 | 后端可替换、业务可 mock | CONFIRMED；新后端不能泄漏驱动类型到核心协议 |
| 会话拥有共享硬件访问边界，线程共享同一 backend mutex | 采样、日志、交互不能竞争 SWD | CONFIRMED；若改变并发模型须重新证明串行化 |
| 历史缓冲与即时硬件访问分离 | Agent 可以在目标运行后查询历史 | CONFIRMED；不把查询演变成无限阻塞的实时读取 |
| 符号解析与硬件适配分离 | ELF/DWARF 分析无需设备 | CONFIRMED；新符号格式保留可离线验证性 |

## 6. Global Invariants

用户已确认的工程要求：默认 Flash 校验、失联恢复与 gap、统一 SWD 同步、子进程清理、稳定 JSON/schema/错误码、硬件测试隔离。详细约束统一由 AGENTS.md 维护，避免两份规则漂移；实现证据与缺口分开记录。

## 7. Major Decisions

### Host-side middleware and backend abstraction

- **Status**：CONFIRMED。
- **Decision**：工具运行于主机；probe-rs 是默认后端，OpenOCD 是可选后端，调用方使用统一 trait。
- **Rationale**：统一 Agent 接入与多种探针/驱动，同时保留离线业务测试边界。
- **Alternatives rejected**：将本仓库当 MCU 固件项目；为每种后端设计独立 Agent 协议。
- **Would revisit if**：新增目标架构无法通过现有 trait 表达，需要用户确认新的边界。
- **Evidence**：README.md、src/backend.rs、src/probe/mod.rs、用户工程契约。

## 8. Non-Goals

- 本仓库不是 MCU 应用固件；test_firmware 是验证资产。
- 不以取消 Flash 校验或掩盖采样缺失换取表面速度。
- 工程基准维护不自动授权接触硬件、发布、提交或推送。

## 9. Strategic Unknowns

| Unknown | 影响 | 状态 / 验证路径 |
|---|---|---|
| 成功硬件路径、恢复、Semihosting 与采样协作的完整验证范围 | 影响所有设备调试 feature 的可靠性声明 | OPEN；独立硬件/模拟集成证据 |
| 无硬件测试与跨平台运行测试的一致边界 | 影响持续回归可信度 | OPEN；隔离真实 attach，建立集成 harness |
| 个人长期知识/资产源 | 影响复用与外部研究路由 | OPEN；PROJECT_RESOURCES.md 中登记 |
