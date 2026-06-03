# spec.md - 编码过程沙盒保险箱与红线契约

> ⓘ 本文件是本次代码重构的「保险圈与硬约束控制中心」。后续 Agent 在进行编码时必须 100% 保持在这些红线圈内。发生任何破坏或规避以下约定的改动，均不可被提交或合并进主分支。

---

## 一、 控制沙盒与严禁篡改红线 (Strict Boundary / Do Not Touch)

绝对禁止 Agent 修改或影响的区域与架构规则：

- **红线 1 [设计文档与共识文件为事实源]**:
  - `context.md`、`AGENTS.md`、`嵌入式调试软件.md` 是本项目的设计宪法。所有代码实现必须与之对齐。
  - 严禁以 "方便实现" 为由偏离已有共识：如自行简化 trait 签名、修改 AGENTS.md 规定的目录结构、引入 tokio 等已否定的依赖（Grill 决策 1 明确用 `std::thread`）。
  - 需求规格书 §四.2 的目录树是此次编码的唯一合法目录结构，不得增删或重命名任何模块。

- **红线 2 [Cargo.toml 依赖白名单]**:
  - 仅允许需求规格书 §4.1 列出的 10 个核心依赖：`clap`、`probe-rs`、`serde` + `serde_json`、`thiserror`、`anyhow`、`log` + `env_logger`、`dirs`、`toml`、`rustyline`。
  - 严禁引入未经用户批准的额外 crate（如 `crossbeam`、`parking_lot`、`tracing`、`indicatif`、`colored` 等）。如果 probe-rs 缺少 feature 导致 `cargo check` 失败，补充 feature 即可，不得换用其他探针库。

- **红线 3 [trait 签名与协议错误码不可变]**:
  - `DebugProbe` trait 的 17 个方法签名（`attach`/`detach`/`flash`/`halt`/`resume`/`step`/`core_count`/`active_core`/`set_breakpoint`/`clear_breakpoint`/`set_watchpoint`/`clear_watchpoint`/`read_mem`/`write_mem`/`read_regs`/`is_halted`/`is_connected`/`try_recover`）严格按设计文档 §3.1 照搬，不得增删参数或修改返回值类型。
  - `LogChannel` trait 的 6 个方法签名（`name`/`open`/`read`/`write`/`is_writable`/`close`）严格按设计文档 §3.3 照搬。
  - 12 个 JSON-Lines 错误码（`E_STATE`/`E_PARAM`/`E_BACKEND`/`E_PROBE`/`E_PROBE_LOST`/`E_FLASH`/`E_NO_DWARF`/`E_NO_SEMIHOSTING`/`E_FLASH_BP_DISABLED`/`E_FLASH_BP_LIMIT`/`E_SERIAL`/`E_INTERNAL`）在 `src/error.rs` 中必须一字不差地声明为 `thiserror` 枚举变体，name 和 code 与设计文档 §5.2 完全一致。

- **红线 4 [禁止 unsafe / 禁止跳过 lint]**:
  - 全项目零 `unsafe` 块（AGENTS.md §3.2 刚性要求）。
  - 严禁任何 `#[allow(clippy::xxx)]`、`#[allow(dead_code)]` 注解绕过 clippy warning。
  - 本次创建的项目骨架必须满足 `cargo clippy --all-targets --all-features -- -D warnings` 零 warning 通过。

---

## 二、 编码设计规范（代码风格偏好对齐）

本次功能开发需要严格执行的代码品质契约：

- **1. 模块声明规则**:
  - 每个源文件的开头遵循 AGENTS.md §4.2 的 `use` 导入顺序：`std` → 第三方 crate → `crate` 内部模块，每组之间空一行。禁止 `use super::*` 和 `use crate::*` 通配导入。
  - 所有 `pub fn` / `pub trait` / `pub struct` 必须有 `///` 文档注释。

- **2. 依赖引入原则**:
  - 仅使用 `Cargo.toml` 中声明的依赖，严禁在代码中通过 `extern crate` 引入未声明依赖。
  - probe-rs 的 features 继承设计文档约定：默认 feature 足够使用（`builtin-targets` + `cmsisdap_v1`），不需要额外 flags。

- **3. 错误类型定义规范**:
  - `src/error.rs` 使用 `thiserror` derive，每个变体附带 `#[error("...")]` 人类可读消息。
  - 每个变体附带 `code: &'static str` 关联常量，值为设计文档 §5.2 的 12 个错误码之一。

---

## 三、 本次开发的硬防崩溃约束

- 1. **trait 方法签名完整性**: `DebugProbe` 和 `LogChannel` trait 的方法签名必须包含所有设计文档规定的参数和返回值类型。关联类型 `BpId`、`WpId` 使用 `type` 别名（`pub type BpId = usize` 等）。
- 2. **clap derive 子命令完整性**: `cli/mod.rs` 中必须定义 4 个枚举变体：`Init`、`Flash`、`Clean`、`Debug`。每个变体附带 `#[clap(about = "...")]` 描述。`debug` 子命令需包含设计文档 §4.2 的完整启动参数列表（`--elf`/`--json`/`--no-flash`/`--break`/`--watch`/`--continue`/`--halt-on-start` 等）。
- 3. **CI 文件四步流水线**: `.github/workflows/ci.yml` 必须包含 `fmt`、`clippy`、`test`、`integration` 四个 job，使用 `actions-rs/toolchain` 固定 Rust 版本 1.95。

---

## 四、 本次规范验收评估核对

任何编码结果在被标记为完成提交至 Pull Request 前，执行 Agent 必须完成以下自检：

- [ ] 没有任何 `// TODO`、`unimplemented!()` 宏体残留（除了空 `{}` 函数体骨架）。
- [ ] `cargo fmt --all -- --check` 返回 exit 0，无格式差异。
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` 返回 exit 0，零 warning。
- [ ] `cargo check` 无错误、无 warning 通过。
- [ ] `git diff` 核实了受改动面积，100% 契合 [task.md](task.md) 规定，无任何出轨改动。
