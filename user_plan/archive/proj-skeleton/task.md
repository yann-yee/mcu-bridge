# task.md - 精细化函数级开发任务清单与执行足迹

> ⓘ 本文件是实现特性的 AI 代理的核心执行手记。每一个步骤都精确写明了受影响文件、拟添加/修改的方法名称。你必须保持在任意时刻**只能有 1 个任务处于 [in-progress] 状态**；完成一步后，运行对应的验证命令并打勾，再推进下一步，保证开发路径 100% 可回溯。

---

## 📌 当前总览
- **源需求文档**: [user_plan/proj-skeleton/proj-skeleton.md](user_plan/proj-skeleton/proj-skeleton.md)
- **最新更新日期**: 2026-06-03 (已归档)
- **整体进度状态**: `completed`

---

## 一、 开发准备与依赖准备 (Preparation)

- [x] **Task 1.1: 确认 Rust 工具链与环境**
  - **描述**: 验证 rustc/cargo/rustfmt/clippy 可用，版本满足 ≥1.95。
  - **本地执行检验命令**: `rustc --version && cargo --version && cargo fmt --version && cargo clippy --version`
  - **当前状态**: `completed`

---

## 二、 项目根文件层 (Cargo.toml + rustfmt.toml + .gitignore)

- [x] **Task 2.1: 创建 Cargo.toml**
  - **受影响文件**: `[Cargo.toml](Cargo.toml)`
  - **函数/属性级实施计划**:
    1. `[package]` 块：`name = "mcu-bridge"`, `version = "0.1.0"`, `edition = "2024"`, `rust-version = "1.95"`, `description = "面向 AI Agent 的嵌入式调试中间件。通过缓冲区解耦 Agent 慢思考与 MCU 快执行。"`, `license = "MIT"`
    2. `[[bin]]` 块：`name = "mcu-bridge"`, `path = "src/main.rs"`
    3. `[dependencies]` 块（10个依赖，按用途分组注释）：
       - CLI: `clap = { version = "4.6", features = ["derive"] }`, `rustyline = "18"`
       - 调试探针: `probe-rs = "0.31"`
       - JSON-Lines 协议: `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`
       - 错误处理: `thiserror = "2"`, `anyhow = "1"`
       - 诊断日志: `log = "0.4"`, `env_logger = "0.11"`
       - 配置与路径: `toml = "0.8"`, `dirs = "6"`
    4. `[profile.release]` 块：`opt-level = 2`, `lto = true`, `codegen-units = 1`
    5. `[profile.dev]` 块：`opt-level = 0`（加速 debug 编译）
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 2.2: 创建 rustfmt.toml**
  - **受影响文件**: `[rustfmt.toml](rustfmt.toml)`
  - **实施计划**: 写入 `edition = "2024"`（其余使用 rustfmt 默认配置）
  - **本地验证命令**: `cargo fmt --all -- --check`
  - **当前状态**: `completed`

- [x] **Task 2.3: 创建 .gitignore**
  - **受影响文件**: `[.gitignore](.gitignore)`
  - **实施计划**: 写入标准 Rust gitignore 条目（`target/`、`Cargo.lock`、`.env`、`*.swp`、`.DS_Store`），**但保留 Cargo.lock 不过滤**（二进制项目应提交 lock 文件）
  - **本地验证命令**: `git check-ignore target/` 应返回 `target/`
  - **当前状态**: `completed`

---

## 三、 类型与错误基础设施层 (src/error.rs + src/config.rs)

- [x] **Task 3.1: 创建 src/error.rs — 错误码枚举**
  - **受影响文件**: `[src/error.rs](src/error.rs)`
  - **函数/属性级实施计划**:
    1. 定义 `use thiserror::Error;`，导入 `thiserror`
    2. 定义 `#[derive(Error, Debug)] pub enum McuBridgeError`，包含以下 12 个变体（含 `#[error("...")]` 人类可读消息和 `code()` 方法返回 `&'static str` 错误码）：
       - `#[error("command not valid in current target state")]` `EState` (code: "E_STATE")
       - `#[error("invalid or missing parameter")]` `EParam` (code: "E_PARAM")
       - `#[error("backend communication failure")]` `EBackend` (code: "E_BACKEND")
       - `#[error("probe disconnected, recovery in progress")]` `EProbe` (code: "E_PROBE")
       - `#[error("probe recovery failed, session ending")]` `EProbeLost` (code: "E_PROBE_LOST")
       - `#[error("flash operation failed")]` `EFlash` (code: "E_FLASH")
       - `#[error("DWARF info needed but not available")]` `ENoDwarf` (code: "E_NO_DWARF")
       - `#[error("operation not supported in semihosting mode")]` `ENoSemihosting` (code: "E_NO_SEMIHOSTING")
       - `#[error("flash breakpoints not enabled")]` `EFlashBpDisabled` (code: "E_FLASH_BP_DISABLED")
       - `#[error("flash breakpoint session limit reached")]` `EFlashBpLimit` (code: "E_FLASH_BP_LIMIT")
       - `#[error("serial port operation failed")]` `ESerial` (code: "E_SERIAL")
       - `#[error("internal error")]` `EInternal` (code: "E_INTERNAL")
    3. 实现 `impl McuBridgeError { pub fn code(&self) -> &'static str { ... } }`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 3.2: 创建 src/config.rs — 配置类型定义**
  - **受影响文件**: `[src/config.rs](src/config.rs)`
  - **函数/属性级实施计划**:
    1. 定义 `use serde::{Deserialize, Serialize};`
    2. 定义 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct ChipConfig`，字段：
       - `pub name: String` — 芯片名称，如 "STM32F407VG"
       - `pub architecture: String` — 架构，如 "cortex-m4"
       - `pub flash_base: u32` — Flash 基址
       - `pub flash_size: u32` — Flash 大小 (bytes)
       - `pub ram_base: u32` — RAM 基址
       - `pub ram_size: u32` — RAM 大小 (bytes)
    3. 定义 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct FlashSection`，字段：
       - `pub name: String`, `pub addr: u32`, `pub len: u32`
    4. 定义 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct FlashOpts`，字段：
       - `pub base: u32`, `pub size: u32`, `pub sections: Vec<FlashSection>`, `pub verify: bool`
    5. 定义 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct DebuggerConfig`，字段：
       - `pub probe: String` — 探针类型: "stlink-v2" | "jlink" | "cmsis-dap" | "ftdi"
       - `pub interface: String` — "swd" | "jtag"
       - `pub speed_khz: u32` — 时钟频率
       - `pub backend: String` — "probe-rs" | "openocd"
    6. 定义 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct SerialConfig`，字段：
       - `pub backend: String` — "rtt" | "uart" | "semihosting" | "auto"
       - `pub port: String` — "auto" | "/dev/ttyACM0" | "COM3"
       - `pub baudrate: u32` — 默认 115200
       - `pub rtt_channel: usize` — 默认 0
    7. 定义 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct WatchConfig`，字段：
       - `pub interval_ms: u64` — 默认 10
       - `pub buffer_size: usize` — 默认 128
    8. 定义 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct RecoveryConfig`，字段：
       - `pub max_retries: u32` — 默认 3
       - `pub retry_delay_ms: u64` — 默认 500
    9. 定义 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct FlashBpConfig`，字段：
       - `pub enabled: bool` — 默认 false
       - `pub max_per_session: u32` — 默认 100
    10. 定义 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct OpenOcdConfig`，字段：
        - `pub cfg_file: String`
        - `pub extra_args: Vec<String>`
    11. 定义 `#[derive(Debug, Clone, Serialize, Deserialize)] pub struct AppConfig`（顶层），字段：
        - `pub chip: ChipConfig`
        - `pub debugger: DebuggerConfig`
        - `pub flash: FlashOpts`
        - `pub serial: SerialConfig`
        - `pub watch: WatchConfig`
        - `pub recovery: RecoveryConfig`
        - `pub flash_bp: FlashBpConfig`
        - `pub openocd: Option<OpenOcdConfig>`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

---

## 四、 核心 trait 定义层 (src/probe/ + src/log/)

- [x] **Task 4.1: 创建 src/probe/mod.rs — DebugProbe trait**
  - **受影响文件**: `[src/probe/mod.rs](src/probe/mod.rs)`
  - **函数/属性级实施计划**:
    1. 定义 `use std::collections::HashMap;` `use std::path::{Path, PathBuf};`
    2. 定义关联类型别名：`pub type BpId = usize;` `pub type WpId = usize;`
    3. 定义 `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum WatchKind { Read, Write, ReadWrite }`
    4. 定义 `pub trait DebugProbe`，包含以下 17 个方法（签名严格按设计文档 §3.1）：
       - `fn attach(&mut self, chip: &ChipConfig) -> Result<(), anyhow::Error>;`
       - `fn detach(&mut self) -> Result<(), anyhow::Error>;`
       - `fn is_connected(&self) -> bool;`
       - `fn try_recover(&mut self) -> Result<(), anyhow::Error>;`
       - `fn flash(&mut self, elf: &Path, opts: &FlashOpts) -> Result<(), anyhow::Error>;`
       - `fn halt(&mut self, core: Option<usize>) -> Result<(), anyhow::Error>;`
       - `fn resume(&mut self, core: Option<usize>) -> Result<(), anyhow::Error>;`
       - `fn step(&mut self, core: Option<usize>) -> Result<(), anyhow::Error>;`
       - `fn core_count(&self) -> usize;`
       - `fn active_core(&self) -> usize;`
       - `fn set_breakpoint(&mut self, addr: u32, core: Option<usize>) -> Result<BpId, anyhow::Error>;`
       - `fn clear_breakpoint(&mut self, id: BpId) -> Result<(), anyhow::Error>;`
       - `fn set_watchpoint(&mut self, addr: u32, len: u32, kind: WatchKind) -> Result<WpId, anyhow::Error>;`
       - `fn clear_watchpoint(&mut self, id: WpId) -> Result<(), anyhow::Error>;`
       - `fn read_mem(&mut self, addr: u32, len: u32, core: Option<usize>) -> Result<Vec<u8>, anyhow::Error>;`
       - `fn write_mem(&mut self, addr: u32, data: &[u8], core: Option<usize>) -> Result<(), anyhow::Error>;`
       - `fn read_regs(&mut self, core: Option<usize>) -> Result<HashMap<String, u64>, anyhow::Error>;`
       - `fn is_halted(&self, core: Option<usize>) -> bool;`
    5. 导入 `crate::config::{ChipConfig, FlashOpts};`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 4.2: 创建 probe-rs backend 空文件**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)`
  - **实施计划**: 写入 `//! probe-rs backend implementation` 注释，声明 `pub struct ProbeRsBackend;` + 空 `impl DebugProbe for ProbeRsBackend { ... }`（方法体均为 `unimplemented!()`)
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 4.3: 创建 OpenOCD backend 空文件**
  - **受影响文件**: `[src/probe/openocd.rs](src/probe/openocd.rs)`
  - **实施计划**: 写入 `//! OpenOCD backend implementation (TCL telnet)` 注释，声明 `pub struct OpenOcdBackend;` + 空 `impl DebugProbe for OpenOcdBackend { ... }`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 4.4: 创建 src/log/mod.rs — LogChannel trait**
  - **受影响文件**: `[src/log/mod.rs](src/log/mod.rs)`
  - **函数/属性级实施计划**:
    1. 定义 `pub trait LogChannel: Send`，包含以下 6 个方法（签名严格按设计文档 §3.3）：
       - `fn name(&self) -> &str;`
       - `fn open(&mut self) -> Result<(), anyhow::Error>;`
       - `fn read(&mut self, buf: &mut [u8]) -> Result<usize, anyhow::Error>;`
       - `fn write(&mut self, data: &[u8]) -> Result<(), anyhow::Error>;`
       - `fn is_writable(&self) -> bool;`
       - `fn close(&mut self) -> Result<(), anyhow::Error>;`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 4.5: 创建三个 LogChannel 实现空文件**
  - **受影响文件**:
    - `[src/log/rtt.rs](src/log/rtt.rs)` — `pub struct RttChannel;` + 空 impl
    - `[src/log/uart.rs](src/log/uart.rs)` — `pub struct UartChannel;` + 空 impl
    - `[src/log/semihosting.rs](src/log/semihosting.rs)` — `pub struct SemihostingChannel;` + 空 impl
  - **实施计划**: 每个文件写入结构体声明和空的 `impl LogChannel for ... { ... }` 块（方法体 `unimplemented!()`）
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

---

## 五、 数据与状态管理层 (src/buffer/ + src/session.rs)

- [x] **Task 5.1: 创建 src/buffer/mod.rs — DebugBuffer + Sample**
  - **受影响文件**: `[src/buffer/mod.rs](src/buffer/mod.rs)`
  - **函数/属性级实施计划**:
    1. 定义 `use std::collections::HashMap;`
    2. 定义 `#[derive(Debug, Clone)] pub struct Sample`，字段严格按设计文档 buffer schema：
       - `pub sn: u64` — 全局序列号
       - `pub tick_us: u64` — μs 时间戳
       - `pub val: u64` — 采样值
       - `pub core: usize` — 核心号
       - `pub bp_flag: bool` — 断点标记
       - `pub gap: bool` — 断连标记
       - `pub regs: Option<HashMap<String, u64>>` — 寄存器快照（仅 bp_flag=true 时有值）
       - `pub old_val: Option<u64>` — watchpoint 触发时旧值
       - `pub new_val: Option<u64>` — watchpoint 触发时新值
    3. 定义 `#[derive(Debug)] pub struct WatchTarget`，字段：
       - `pub id: usize`
       - `pub label: String`
       - `pub addr: u32`
       - `pub size: u32`
       - `pub kind: crate::probe::WatchKind`
    4. 定义 `#[derive(Debug)] pub struct DebugBuffer`，字段：
       - `pub targets: Vec<WatchTarget>`
       - `pub samples: std::collections::HashMap<usize, Vec<Sample>>` — key 为 watch id
       - `pub capacity: usize` — 每个 target 的 ring buffer 容量
       - `pub global_sn: u64` — 全局递增序列号
    5. 实现 `impl DebugBuffer`：
       - `pub fn new(capacity: usize) -> Self`
       - `pub fn push_sample(&mut self, watch_id: usize, mut sample: Sample)`
    6. 空壳 `pub struct SerialMonitor;` + `impl SerialMonitor { pub fn new(...) -> Self { todo!() } }`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 5.2: 创建 src/buffer/serial.rs — SerialMonitor 空壳**
  - **受影响文件**: `[src/buffer/serial.rs](src/buffer/serial.rs)`
  - **实施计划**: 写入 `//! SerialMonitor — 日志通道接收线程` + 空壳 `pub struct SerialMonitor;`（后续 Phase 实现）
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 5.3: 创建 src/session.rs — SessionState 状态机**
  - **受影响文件**: `[src/session.rs](src/session.rs)`
  - **函数/属性级实施计划**:
    1. 定义 `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum SessionState { Halted, Running, Recovering }`
    2. 定义 `pub struct Session`，字段（空壳，后续实现）：
       - `pub state: SessionState`
       - `pub chip_name: String`
       - `pub core_count: usize`
       - `pub pc: Option<u32>`
       - `pub bp_count: usize`
       - `pub watch_count: usize`
    3. 实现 `impl Session { pub fn new() -> Self { ... } }`（初始态 `Halted`）
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

---

## 六、 CLI 与入口点层 (src/main.rs + src/cli/)

- [x] **Task 6.1: 创建 src/cli/mod.rs — clap derive CLI 定义**
  - **受影响文件**: `[src/cli/mod.rs](src/cli/mod.rs)`
  - **函数/属性级实施计划**:
    1. 导入 `use clap::{Parser, Subcommand};`
    2. 定义 `#[derive(Parser)] #[command(name = "mcu-bridge", version, about = "面向 AI Agent 的嵌入式调试中间件")] pub struct Cli`：
       - `#[command(subcommand)] pub command: Commands`
    3. 定义 `#[derive(Subcommand)] pub enum Commands`：
       - `#[command(about = "初始化芯片配置 (.debugger/chip.toml)")] Init { #[arg(long)] chip: String, #[arg(long)] debugger: Option<String>, #[arg(long)] interface: Option<String> }`
       - `#[command(about = "烧录 ELF 固件到目标芯片")] Flash { #[arg(long)] elf: PathBuf, #[arg(long)] verify: bool, #[arg(long)] chip: Option<String> }`
       - `#[command(about = "清理缓存目录 (~/.mcu_bridge/)")] Clean { #[arg(long)] all: bool, #[arg(long)] older_than: Option<String> }`
       - `#[command(about = "启动调试会话 (REPL 或 JSON-Lines)")] Debug { #[arg(long)] elf: PathBuf, #[arg(long)] config: Option<PathBuf>, #[arg(long)] json: bool, #[arg(long)] no_flash: bool, #[arg(long)] verify: bool, #[arg(long)] backend: Option<String>, #[arg(long)] enable_flash_bp: bool, #[arg(long = "break", value_delimiter = ',')] break_at: Vec<String>, #[arg(long = "watch", value_delimiter = ',')] watch_targets: Vec<String>, #[arg(long)] continue_: bool, #[arg(long)] halt_on_start: bool, #[arg(long)] sampling_interval: Option<u64>, #[arg(long)] serial_port: Option<String>, }`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 6.2: 创建子命令空壳文件**
  - **受影响文件**:
    - `[src/cli/init.rs](src/cli/init.rs)` — `pub fn handle(cmd: &InitArgs) -> anyhow::Result<()>`
    - `[src/cli/flash.rs](src/cli/flash.rs)` — `pub fn handle(cmd: &FlashArgs) -> anyhow::Result<()>`
    - `[src/cli/clean.rs](src/cli/clean.rs)` — `pub fn handle(cmd: &CleanArgs) -> anyhow::Result<()>`
    - `[src/cli/debug.rs](src/cli/debug.rs)` — `pub fn handle(cmd: &DebugArgs) -> anyhow::Result<()>`
  - **实施计划**: 每个文件写入对应的参数 struct + 空 `handle` 函数（`todo!()`）
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 6.3: 创建 src/main.rs — 入口点**
  - **受影响文件**: `[src/main.rs](src/main.rs)`
  - **函数/属性级实施计划**:
    1. 声明所有模块：`mod cli; mod probe; mod buffer; mod log; mod session; mod config; mod error;`
    2. 初始化 `env_logger`（由 `env_logger::init()` 延迟到首次使用时）
    3. 使用 `clap::Parser::parse()` 解析 CLI
    4. `match` 分发到对应子命令处理函数
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

---

## 七、 CI/CD 与全局验收 (CI + Final Verification)

- [x] **Task 7.1: 创建 .github/workflows/ci.yml**
  - **受影响文件**: `[.github/workflows/ci.yml](.github/workflows/ci.yml)`
  - **实施计划**:
    1. `name: CI`
    2. `on: [push, pull_request]`
    3. 定义 4 个 job：
       - `fmt`: `runs-on: ubuntu-latest` → `actions-rs/toolchain` (rust 1.95) → `run: cargo fmt --all -- --check`
       - `clippy`: `runs-on: ubuntu-latest` → `actions-rs/toolchain` → `run: cargo clippy --all-targets --all-features -- -D warnings`
       - `unit-test`: `runs-on: ubuntu-latest` → `actions-rs/toolchain` → `run: cargo test --lib`
       - `integration`: `runs-on: ubuntu-latest` → `actions-rs/toolchain` → Docker 启动 OpenOCD 0.12 容器 → `run: cargo test --test integration`（暂用 `echo "integration tests: skipped (no test binary yet)"` 占位）
  - **本地验证命令**: `cargo check`（验证 CI 文件存在即可，YAML 语法人工检查）
  - **当前状态**: `completed`

- [x] **Task 7.2: 全量验收 — cargo check**
  - **描述**: 编译检查通过，零 error / 零 warning。
  - **执行命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 7.3: 全量验收 — cargo fmt**
  - **描述**: 代码格式化检查。
  - **执行命令**: `cargo fmt --all -- --check`
  - **当前状态**: `completed`

- [x] **Task 7.4: 全量验收 — cargo clippy**
  - **描述**: Lint 零容忍，所有 target 和 feature 均通过。
  - **执行命令**: `cargo clippy --all-targets --all-features -- -D warnings`
  - **当前状态**: `completed`

- [x] **Task 7.5: 全量验收 — cargo run -- --help**
  - **描述**: 入口点输出包含 `init`/`flash`/`clean`/`debug` 四个子命令。
  - **执行命令**: `cargo run -- --help`
  - **当前状态**: `completed`

- [x] **Task 7.6: 文件完整性验收**
  - **描述**: 逐条检查需求规格书 §五 的 7 个文件中的类型/结构体/trait 声明存在性。
  - **当前状态**: `completed`

- [x] **Task 7.7: 更新 context.md**
  - **描述**: 将本次编码工程中敲定的 Grill 决策（std::thread/thiserror+anyhow/log+env_logger）增量合并到 `context.md` §四.1 架构决策，并追加维护日志。
  - **当前状态**: `completed`
