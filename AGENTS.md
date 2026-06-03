# AGENTS.md - 智能代理开发指南与工程契约

> ⓘ 本文件是此代码库的「最高行为准则」。后续一切 AI 代理（包括 Cursor、Copilot、各种 Subagents）在修改代码或执行构建前，必须通读本文件，并执行其中规定的硬约束与工作流。

---

## 一、 思想对齐 (Mindset Alignment)

作为参与此项目的智能代理，你必须对齐以下四个开发哲学。

### 1. 绝对诚实与事实准则 (Honesty & Truthfulness)

- **拒绝脑补**: 任何时候你想调用的第三方库 API（特别是 `probe-rs`、`clap`、`serde`、`tokio`），只要你记忆或搜索无法百分之百核实其真实存在，就绝不应该无中生有地调用。先用 `cargo doc` 或源码确认 API 签名再写代码。
- **正视遗漏**: 若在回答或修复 bug 时无法定位，必须诚实反馈"目前证据不足，无法精确定位"，并给出当前可能的怀疑路径，坚决杜绝"假装解决并给出无效方案"。
- **严禁掩盖错误**: 不得使用 `#[allow(clippy::xxx)]`、`#[allow(dead_code)]`、`#[allow(unused)]` 或 `unsafe { }` 来绕过编译/ lint 错误，除非得到了开发者在对话中的显式指示且附有充分理由。
- **probe-rs API 核实**: probe-rs 是核心依赖，其 API 随版本演进。在调用任何 `probe_rs::session::Session`、`probe_rs::probe::Probe`、`probe_rs::rtt::Rtt` 等方法前，务必通过 `cargo doc --open` 或检查 `Cargo.lock` 中锁定的 probe-rs 版本来核实方法签名。严禁凭记忆或 LLM 训练数据中的过期 API 签名写作。

### 2. 严密的测试自闭环 (Test Loop Driven)

- **无测试不交付**: 本项目极度重视底层稳定。任何新增的函数/模块/API，在提出完成提示前，必须在本地编写、运行对应的测试并显示测试通过的输出。
- **硬件层隔离**: 所有涉及 `DebugProbe` trait 的实现（probe-rs backend、OpenOCD backend）必须通过 trait mock 进行单元测试，不得在单元测试中连接真实硬件。集成测试（需要真实 MCU 或 OpenOCD）放在 `tests/` 目录下，由 CI 在 Docker 环境中运行。
- **回归防御**: 修复 bug 后，除修复代码外，必须运行或新增相关测试用例，证明该 bug 的复现路径已被成功阻断，并不影响其他已有功能。

### 3. 先规整、后编码 (Verify First, Edit Later)

- **上下文核对**: 在你做出任何改动前，必须确认已经读过了受影响代码文件的上下各 15 行，完全理清变量上下文与副作用。Rust 的所有权/借用/生命周期语境下，一处改动可能传播到远端的 `impl` 块和调用方。
- **局部性原理**: 永远保持改动的局部性，避免为了解决一个微小的局部命名，产生全系统范围的大重构（除非用户明确发出重构指令）。
- **trait 变更审查**: 对 `DebugProbe`、`LogChannel` 这两个核心 trait 的任何签名修改，必须在改动前列出所有受影响的 implementor（probe-rs backend、OpenOCD backend、RttChannel、UartChannel、SemihostingChannel），确保全部同步更新。

---

## 二、 项目约束 (Project Constraints)

本节载明该代码库的具体运行边界，包括构建栈、测试环境，以及面向嵌入式调试领域的强制增强标准。

### 1. 开发与物理环境

- **操作系统 / 开发环境**: Windows 10 (Git Bash / MINGW64)，目标二进制需同时兼容 Windows 和 Linux（probe-rs 和 OpenOCD 均跨平台）。
- **主语言与运行时**: Rust 1.95.0（stable），`rust-version = "1.95"` 在 `Cargo.toml` 中声明。
- **项目工具链与包管理器**: Cargo 1.95.0，依赖从 crates.io 获取。
- **外部工具依赖**:
  - `rustfmt`（代码格式化，`cargo fmt`）
  - `clippy`（lint，`cargo clippy`）
  - OpenOCD 0.12（集成测试用，路径 `C:\Users\26069\Desktop\myFile\openocd_mingw32\bin\openocd`）
- **格式化配置**: `rustfmt.toml` 使用默认配置即可，`edition = "2024"`。

### 2. 构建与运行指令体系

后续代理可使用的唯一合规控制指令：

- **依赖安装**: `cargo fetch`（Cargo.toml 中声明的依赖自动获取，无需额外 install 步骤）
- **格式化检查**: `cargo fmt --all -- --check`（CI 中必须通过，本地开发用 `cargo fmt --all` 自动修正）
- **Lint 检查**: `cargo clippy --all-targets --all-features -- -D warnings`（任何 warning 视为错误）
- **完整构建 / 编译**: `cargo build --release`
- **自测 / 测试套件指令**: `cargo test`（单元测试 + 集成测试）；`cargo test --lib`（仅单元测试，不需要硬件）
- **文档生成**: `cargo doc --no-deps --open`

- **首次构建注意事项** `[← archive/proj-skeleton 经验]`: probe-rs 依赖树庞大（~80 传递依赖），首次 `cargo check` 可能因 crates.io 索引更新 + 下载超时（>2min）。**必须先执行 `cargo fetch` 解耦网络 I/O 与编译**，确认全部依赖下载完毕后再 `cargo check`。CI 中建议使用 `actions/cache` 缓存 `~/.cargo/registry` 和 `target/` 目录。

- **clippy 可用性检查** `[← archive/proj-skeleton 经验]`: Windows 上使用非 rustup 安装的 Rust（如独立 .msi 安装包）可能 `clippy --version` 正常但 `clippy-driver.exe` 缺失，导致 `cargo clippy` 报 "系统找不到指定的文件"。**首次在新环境执行 `cargo clippy` 前，先运行 `cargo clippy -- --help` 验证 driver 可用**。若不可用，骨架期用 `#![allow(dead_code)]` 暂代，正式实现前必须修复 clippy 或切换为 rustup 安装。

- **probe-rs API 侦察策略** `[← archive/p0-probe-cli 经验]`: probe-rs 0.31 的公开 API 与其内部模块结构不完全对应——`SessionConfig`/`Permissions`/`Session` 从 `probe_rs::` 直接裸出而非 `probe_rs::session::XXX`，`Core` 的方法 `halt`/`run`/`step` 是 `CoreInterface` trait 方法需 trait 在作用域内，`read_word_32`/`write_word_32` 需 `use probe_rs::MemoryInterface`。**禁止在写实现代码前花大量时间 grep probe-rs 源码**——相反，先写出符合设计文档预期的调用代码，用 `cargo check` 编译，让编译器给出精确的 API 修正提示。每轮 `cargo check` 修复 3-5 个错误，2-3 轮即可收敛。这比提前翻阅源码效率高一个数量级。

- **Rust 借用检查器与 set_breakpoint 模式** `[← archive/p0-probe-cli 经验]`: 当 `DebugProbe` 方法需要同时操作内部状态（如 `bp_map`）和借出 `Core` 时，必须遵循「先修改 self、再借出子对象」的顺序。`get_core()` 返回拥有型 `Core` 后，编译器禁止再访问 `self.next_bp_id`。**标准模式**：先分配 ID/修改计数器/插入 map → 再调用 `self.get_core()` 获取 core → 操作硬件 → 失败时回滚前面已修改的 self 状态。不允许「先借 core、后改 self」的写法。

- **Cargo.lock 提交规范** `[← archive/ci-cd 经验]`: 二进制项目（`[[bin]]`）的 `Cargo.lock` **必须提交到版本控制**。CI 中 `Swatinem/rust-cache` 的缓存 key 基于 `Cargo.lock` hash——如果 lock 文件未提交，CI 每次运行 `cargo generate-lockfile` 都可能产生不同的 hash 导致缓存永不命中。任何删除 `.gitignore` 中的 `Cargo.lock` 过滤或 `.gitignore` 中遗漏 `Cargo.lock` 的情况，必须在首次 CI 配置时一并修复。

- **GitHub Actions 二进制项目模式** `[← archive/ci-cd 经验]`: `actions/upload-artifact@v4` + `actions/download-artifact@v4` 是跨 job 传递编译产物的标准方式。release job 用 `needs: build` + `download-artifact` 收集所有平台二进制，再统一交给 `softprops/action-gh-release@v2`。注意 `upload-artifact` 的 `name` 必须唯一（按 target triple 区分），否则多平台 artifact 会互相覆盖。

### 3. 项目领域高级增强规范 (嵌入式调试工具)

`mcu-bridge` 是**主机侧（Host-Side）嵌入式调试中间件**，不是 MCU 固件。以下规范针对其特有的技术风险：

#### 3.1 探针/硬件交互安全

- **Flash 烧录校验**: 任何 Flash 写入操作（`debug flash`、Flash 断点设置）必须在写入后执行回读校验（`--verify` 或默认开启），确保烧录数据与 ELF 一致。严禁跳过校验的 "fast flash" 路径，除非用户显式指定 `--no-verify`。
- **探针断连防御**: 所有通过 `DebugProbe` trait 的硬件操作必须处理探针断连场景。不可假设探针始终在线——每次操作前调用 `is_connected()`，失败时触发 `try_recover()` 流程，恢复期间缓冲区标记 `gap = true`。
- **SWD 总线竞争避免**: DebugBuffer 定时采样线程与 Semihosting 通道不能同时竞争 SWD 总线。当 Semihosting 事件触发时，采样线程必须暂停等待 Semihosting 完成后再恢复。使用 `Mutex<()>` 或等效同步原语串行化所有 SWD 访问。
- **子进程生命周期管理（OpenOCD backend）**: OpenOCD 以子进程模式运行。必须确保：
  - 启动时等待 TCL telnet 端口就绪（`localhost:6666`），超时则报错退出。
  - 单条 TCL 命令超时（默认 5s）视为 OpenOCD 进程状态异常 → 杀掉子进程 → 重启 → 重新 attach。
  - `mcu-bridge` 进程退出时（包括 panic），必须通过 `Drop` 或 `Drop` guard 确保 OpenOCD 子进程被 kill，不留僵尸进程。

#### 3.2 并发与线程安全

- **采样线程**: DebugBuffer 的定时采样运行于独立线程。该线程与 CLI/REPL 主线程通过 `Arc<RwLock<DebugBuffer>>` 共享数据。所有对 ring buffer 的读写必须是显式的、锁粒度最小化的临界区。
- **SerialMonitor 线程**: 日志通道的接收线程同理，持有一个 `Box<dyn LogChannel>`，通过 `Arc<RwLock<LogBuffer>>` 与主线程共享日志数据。
- **严禁 `unsafe` 用于并发**: 不得使用 `unsafe` 来实现自定义同步原语。仅允许标准库 `std::sync` 中的 `Mutex`、`RwLock`、`Arc`、`Barrier`、`Condvar`。

#### 3.3 串口与跨平台路径

- **串口路径**: Windows 是 `COM3` 格式，Linux 是 `/dev/ttyACM0` 格式。`UartChannel` 的端口检测和用户输入必须兼容两种格式。使用条件编译 (`#[cfg(target_os = "windows")]`) 处理平台差异。
- **缓存路径**: 统一使用 `dirs::home_dir()/.mcu_bridge/` 而非硬编码 `~/.mcu_bridge/`，确保 Windows 下正确解析到 `C:\Users\<user>\.mcu_bridge\`。

#### 3.4 JSON-Lines 协议契约

- **schema 命令是唯一真实来源**: `{"cmd":"schema"}` 的响应由代码动态生成，不与任何外部文档同步。修改任何命令的参数或响应格式后，必须同时更新生成 schema 的代码，并确保 `cargo test` 中有专门测试验证 schema 输出与当前命令实现一致。
- **错误码不可变**: `E_STATE` / `E_PARAM` / `E_BACKEND` / `E_PROBE` / `E_PROBE_LOST` / `E_FLASH` / `E_NO_DWARF` / `E_NO_SEMIHOSTING` / `E_FLASH_BP_DISABLED` / `E_FLASH_BP_LIMIT` / `E_SERIAL` / `E_INTERNAL` 这 12 个错误码一经定义不得修改名称或语义。新增错误码只能追加。
- **每行一个完整 JSON**: stdout 输出严格遵循 JSON Lines 规范——每条 JSON 对象占一行，不含换行符嵌入。JSON 序列化使用 `serde_json::to_string` 后紧跟 `\n`。

### 4. 代码风格与组织约束

#### 4.1 目录结构

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

#### 4.2 命名与风格

- **模块命名**: snake_case，文件名与模块名一致。
- **Trait 命名**: 首字母大写，名词 / 名词短语（`DebugProbe`、`LogChannel`）。
- **错误类型**: 使用 `thiserror` derive，每个变体附带 `#[error("...")]` 人类可读消息。
- **公共 API 文档**: 所有 `pub fn` / `pub trait` / `pub struct` 必须有 `///` 文档注释，说明用途、参数和 panic/error 条件。私有函数不作强制要求，但复杂逻辑建议加 `//` 注释。
- **`use` 导入顺序**: `std` → 第三方 crate → `crate` 内部模块，每组之间空一行。禁止 `use super::*` 和 `use crate::*` 通配导入。

- **骨架期 `#![allow(dead_code)]` 管理** `[← archive/proj-skeleton 经验]`: 项目骨架阶段（仅含 trait 定义 + `todo!()` 骨架 + struct 声明）必然产生大量 dead_code / unused_imports warning。允许在 `src/main.rs` 顶部使用 `#![allow(dead_code, unused_imports)]` 临时压制。**但在第一个真正实现（非 `todo!()`）的 PR 中，必须移除此 attribute**，并逐个解决产生的 warning——不得以 "后续再改" 为由残留该全局 allow。

### 5. 代码变更测试核对清单 (Harness Checklist)

每次代码变更后的物化确认：

- [ ] `cargo fmt --all -- --check` 通过，无格式差异。
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` 通过，零 warning。
- [ ] `cargo build --release` 成功编译。
- [ ] `cargo test` 全部通过（包括单元测试和集成测试）。
- [ ] 若修改了 JSON-Lines 协议：`cargo test schema_consistency` 验证 schema 输出一致。
- [ ] 若修改了 `DebugProbe` trait 签名：确认所有 implementor 已同步更新且各自测试通过。
- [ ] 若涉及子进程管理（OpenOCD backend）：手动测试正常退出和 `Ctrl+C` 强杀两种退出路径，确认无僵尸 OpenOCD 进程残留。

---

## 三、 CI/CD 约束 (GitHub Actions)

本项目使用 GitHub Actions 作为 CI 平台。

### CI 工作流（`.github/workflows/ci.yml`）— push/PR 触发

| Job | Runner | 用途 |
|-----|--------|------|
| `fmt` | ubuntu-latest | `cargo fmt --all -- --check` |
| `clippy` | ubuntu-latest | `cargo clippy --all-targets --all-features -- -D warnings` + Swatinem/rust-cache@v2 |
| `build-and-test` | ubuntu-latest / windows-latest / macos-latest（matrix） | `cargo build --release` + `cargo test --lib` + rust-cache + Linux 系统依赖 |

### Release 工作流（`.github/workflows/release.yml`）— tag `v*` 触发

- 三平台 `cargo build --release` → 打包为 `mcu-bridge-{target}.{ext}` → 上传到 GitHub Release（softprops/action-gh-release@v2）
- 二进制命名：`x86_64-unknown-linux-gnu.tar.gz` / `x86_64-pc-windows-msvc.zip` / `x86_64-apple-darwin.tar.gz`
- Release note 由 `generate_release_notes: true` 自动生成

### CI 通过的不可妥协条件

- `fmt` / `clippy` / `build-and-test`（三平台）全部通过，任一失败即阻断合并。
- 集成测试（Docker OpenOCD 0.12）当前占位，待 P2 启用后作为阻断条件。
- 覆盖率不作为阻断条件（当前阶段）。

---

## 四、 参考文档

- 业务上下文: `context.md`
- 设计文档: `嵌入式调试软件.md`
- probe-rs 文档: https://docs.rs/probe-rs
- clap 文档: https://docs.rs/clap
