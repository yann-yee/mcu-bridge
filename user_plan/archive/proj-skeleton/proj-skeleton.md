# 需求规格说明书：项目骨架搭建（P0 基础设施）

> ⓘ 本文档已经过 "Understanding (Grill Me)" 技能对齐，记录了逻辑共识。本文件已于 [user_plan/proj-skeleton/proj-skeleton.md](user_plan/proj-skeleton/proj-skeleton.md) 归档。实现该功能的后续 Agent 必须全盘以此文档为验收终点。

---

## 一、 功能概览与基本愿景 (User Story & Goal)

- **新功能背景**: `mcu-bridge` 是一个全新的 Rust 项目。在开始任何业务逻辑之前，必须先创建 Cargo 项目骨架——包括依赖声明、源码模块目录树、CI 配置。这是实现 P0 目标（DebugProbe trait + CLI 框架）的前提。
- **用户故事 (User Story)**: 作为一名开发者，我想要 `cargo check` 通过一个完整的模块骨架 + CI 流水线就位，以便后续任何 Agent 可以立即在已有模块文件中填充 trait 实现和 CLI 逻辑。
- **关联已有的技术链**:
  - `context.md`：业务共识档案，定义词汇表和 P0-P3 里程碑
  - `AGENTS.md`：工程契约，定义目录结构、`rustfmt`/`clippy` 规则、CI 步骤
  - `嵌入式调试软件.md`：设计文档，定义 `DebugProbe`/`LogChannel` trait 签名和模块架构

---

## 二、 极限对齐的业务流程 (Consensus Flow & Sequence)

> 记录烤问阶段彻底敲定的技术决策与方案选择。

### 1. 标准顺畅流 (Happy Path)

1. 创建 `Cargo.toml`（含 `[dependencies]`、`[package]`、`[profile.release]`），声明 `rust-version = "1.95"`
2. 按 AGENTS.md 规定的目录树创建 `src/` 下所有模块文件（含 `mod.rs`、trait 骨架定义、空实现文件）
3. 创建 `rustfmt.toml`（edition = "2024"）
4. 创建 `.github/workflows/ci.yml`（fmt → clippy → unit test → integration test）
5. 创建 `.gitignore`
6. `cargo check` 编译通过，零 warning

### 2. 异常与阻断流 (Exception Handlings)

- **依赖版本冲突**: `cargo update` 失败时，锁定 `Cargo.toml` 中核心依赖（probe-rs、clap、serde）的精确版本号范围
- **probe-rs feature 不完整**: 如果 `cargo check` 时 probe-rs 缺少必需的 feature（如 RTT、DWARF），补充 `features = [...]` 声明并重新 check

---

## 三、 烤问决策记录 (Grill Decisions)

本需求在 Understanding 阶段经历了三轮极限追问。以下为所有敲定的技术分歧点：

### 🔧 决策 1：异步运行时 → `std::thread` + `mpsc`（否定 tokio）

- **理由**: 线程数量静态（采样 + 日志 + 主线程共 3 个），probe-rs 是同步 API，tokio 的 M:N 调度是过度设计。编译更快、二进制更小。
- **否定方案**: tokio（编译慢、异步优势在本项目中用不上）

### 🔧 决策 2：错误处理 → `thiserror`（模块内） + `anyhow`（CLI 层映射）

- **理由**: 12 个 JSON-Lines 错误码是协议层概念，与内部模块失败原因不是一对一关系。`anyhow` 在 CLI 层集中收敛映射更干净；各模块 `thiserror` 枚举自包含、未来重构成 workspace 时天然独立。
- **否定方案**: 全项目统一 `thiserror` 枚举（模块间耦合过高、错误码映射混乱）

### 🔧 决策 3：诊断日志 → `log` + `env_logger`（否定 tracing、否定 eprintln!）

- **理由**: probe-rs 内部使用 `log` crate，选 `log` 可直接通过 `RUST_LOG=probe_rs=debug` 看到探针库内部诊断。`env_logger` 从环境变量读取过滤级别，Human 模式 stderr 彩色、JSON 模式 stdout 协议分离。
- **否定方案**: tracing（async span 优势用不上）+ 手工 eprintln!（无级别过滤）

---

## 四、 技术契约定义 (Technical Contract)

### 4.1 Cargo.toml 核心依赖

| 依赖 | 用途 | 关键 feature |
|------|------|-------------|
| `clap` | CLI 框架 | `derive` |
| `probe-rs` | 调试探针 API | `rtt`, `dwarf` |
| `serde` + `serde_json` | JSON-Lines 协议 | `derive` |
| `thiserror` | 模块内错误枚举 | — |
| `anyhow` | CLI 层错误链式包装 | — |
| `log` + `env_logger` | 诊断日志 | — |
| `dirs` | 跨平台 `~/.mcu_bridge/` | — |
| `toml` | 配置文件解析 | — |
| `rustyline` | Human REPL 交互 | — |

### 4.2 目录结构（与 AGENTS.md §4.1 严格对齐）

```
src/
├── main.rs              # 入口点，CLI 子命令分发 (clap)
├── cli/
│   ├── mod.rs           # CLI 定义 (clap derive)
│   ├── init.rs          # init 子命令
│   ├── flash.rs         # flash 子命令
│   ├── clean.rs         # clean 子命令
│   └── debug.rs         # debug 子命令（含 REPL + JSON 模式）
├── probe/
│   ├── mod.rs           # DebugProbe trait 定义
│   ├── probe_rs.rs      # probe-rs backend
│   └── openocd.rs       # OpenOCD backend
├── buffer/
│   ├── mod.rs           # DebugBuffer (ring buffer + 采样线程)
│   └── serial.rs        # SerialMonitor (日志通道接收线程)
├── log/
│   ├── mod.rs           # LogChannel trait 定义
│   ├── rtt.rs           # RttChannel
│   ├── uart.rs          # UartChannel
│   └── semihosting.rs   # SemihostingChannel
├── session.rs           # Session 状态机管理
├── config.rs            # 配置加载 (TOML → Config struct)
└── error.rs             # 统一错误类型 + 错误码映射
```

### 4.3 CI 流水线（与 AGENTS.md §三 严格对齐）

```yaml
jobs:
  fmt:    cargo fmt --all -- --check
  clippy: cargo clippy --all-targets --all-features -- -D warnings
  test:   cargo test --lib
  integration: cargo test --test integration  # Docker OpenOCD 0.12
```

---

## 五、 验收断言与 Harness 测试指标 (Definition of Done)

> 绝对禁止空洞通过。以下每条都必须通过命令验证。

- [ ] **1. 骨架编译断言**: `cargo check` 无错误、无 warning 通过。
- [ ] **2. 格式断言**: `cargo fmt --all -- --check` 返回 exit 0（无格式差异）。
- [ ] **3. Lint 零容忍断言**: `cargo clippy --all-targets --all-features -- -D warnings` 返回 exit 0，零 warning。
- [ ] **4. 模块完整性断言**:
  - `src/probe/mod.rs` 中存在 `pub trait DebugProbe { }` 声明（含 `attach`/`detach`/`flash`/`halt`/`resume` 等核心方法签名）
  - `src/log/mod.rs` 中存在 `pub trait LogChannel { }` 声明（含 `open`/`read`/`write`/`close` 方法签名）
  - `src/buffer/mod.rs` 中存在 `pub struct DebugBuffer` 和 `pub struct Sample` 声明
  - `src/error.rs` 中存在 12 个错误码对应的 `thiserror` 枚举变体
  - `src/config.rs` 中存在 `pub struct ChipConfig` / `pub struct FlashOpts` / `pub struct SerialConfig` / `pub struct WatchConfig` 声明
  - `src/session.rs` 中存在 `pub enum SessionState { Halted, Running, Recovering }` 声明
  - `src/cli/mod.rs` 中存在 `clap` derive 的 CLI 定义，含 `Init`/`Flash`/`Clean`/`Debug` 四个子命令
- [ ] **5. CI 文件存在断言**: `.github/workflows/ci.yml` 文件存在，含 fmt/clippy/test/integration 四个 job。
- [ ] **6. 入口点断言**: `cargo run -- --help` 输出包含 `init`/`flash`/`clean`/`debug` 四个子命令。
