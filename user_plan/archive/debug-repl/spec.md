# spec.md — Debug REPL 编码红线契约

> ⓘ 本文件是本次编码的"保险圈与硬约束控制中心"。后续 Agent 在进行编码时必须 100% 保持在这些红线圈内。发生任何破坏或规避以下约定的改动，均不可被提交或合并进主分支。

---

## 一、 控制沙盒与严禁篡改红线 (Strict Boundary / Do Not Touch)

绝对禁止 Agent 修改或影响的区域与架构规则：

- **红线 1 [DebugProbe trait 签名冻结]**: `[src/probe/mod.rs](src/probe/mod.rs)` 中的 `DebugProbe` trait 不得因本次需求而修改（包括新增、删除、重命名、改参数）。所有 18 个方法签名保持原样。
- **红线 2 [probe-rs backend 冻结]**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)` 中的 `attach/detach/flash/halt/resume/step/breakpoint/mem/regs` 实现不得修改。所有方法已实现且通过 13 个测试检验。
- **红线 3 [main.rs CLI 路由冻结]**: `[src/main.rs](src/main.rs)` 中 `Commands::Debug` 的 15 字段解构+传入 `DebugArgs` 的代码已经存在，**不得修改**。新代码只需读取 `DebugArgs` 中的字段。
- **红线 4 [Cargo.toml 无新增依赖]**: 不得新增任何外部 crate。`rustyline` 已在依赖中（版本 18）。
- **红线 5 [CLI Commands 枚举完整保留]**: `Init` / `Flash` / `Clean` / `Debug` 四个变体不可删除或重命名。
- **红线 6 [已有测试全量保护]**: 已有的 17 个测试（10 probe-rs + 3 flash + 2 openocd + 2 路由）不得因新代码被修改或删除。新代码必须全部通过。

---

## 二、 实现红线 (Implementation Constraints)

- **红线 7 [`Session::new()` 保留]**: 已有 `Session::new(chip_name: String)` 构造函数必须保留，**只能追加 `#[deprecated]` 属性**，不得删除或修改签名。其他模块可能使用它。
- **红线 8 [Command::parse 不可 panic]**: `Command::parse()` 必须返回 `Result<Self, String>`，在所有输入（包括空字符串、乱码、1000 个空格）下都不得 panic、不得 `unwrap()`、不得 `todo!()`。
- **红线 9 [禁止通配导入]**: 测试模块 `mod tests` 不得使用 `use super::*`，必须显式导入每个使用项。
- **红线 10 [Round 2 参数仅解析不实现]**: `handle()` 中以下 CLI 参数必须被正确接收和解构，但**其功能逻辑不实现**：
  - `--json` — 不实现 JSON-Lines 协议循环
  - `--break-at` — 不实现启动时自动设断点
  - `--watch` — 不实现变量观测
  - `--continue_` — 不实现启动后自动 continue
  - `--halt-on-start` — 不实现启动 halt
  - `--sampling-interval` — 不实现采样配置
  - `--serial-port` — 不实现串口
  - `--enable-flash-bp` — 不实现 Flash 断点
  - `--no-flash` — 不实现 REPL 内烧录
  - `--config` — 不实现自定义配置路径
  - `--verify` — 不实现烧录校验
  - `--backend` — 本轮仅支持 probe-rs
- **红线 11 [状态守卫强制]**: REPL 主循环中，每条命令执行前必须检查 `valid_states()`。状态不合法时 `println!` 返回错误消息并 `continue`，不得直接 panic 或 `unwrap_err()`。
- **红线 12 [Drop 防泄露]**: `handle()` 退出或 panic 时，`Session` 的 `Drop` 中应确保 `backend.detach()` 被调用。当前 `ProbeRsBackend` 的 `detach()` 仅清空状态（不会 I/O 失败），所以直接调用即可。若将来改为 OpenOCD backend，需要更完善的 Drop guard。

---

## 三、 编码设计规范 (Coding Style)

- **1. 格式化输出前缀**: 成功用 `[OK]`，错误用 `[ERROR]`，断点命中用无前缀 `** ... **`。所有输出写入 `println!`，轮询/日志输出写入 `eprintln!`。
- **2. 测试模块结构**: `#[cfg(test)]` 包裹的 `mod tests` 放在文件末尾。每个 `use` 导入显式指定，禁止 `use super::*`。测试函数使用 `#[test]` 属性。
- **3. let 绑定类型标注**: 对于非显而易见的类型，标注类型（如 `let addr: u32 = ...`）以提升可读性。
- **4. 错误消息人性化**: 用户可见的错误消息用中文或英文均可，但要保持一致。建议使用英文（与现有 `flash.rs` 风格一致）。
- **5. 无残留 todo!()**: `src/cli/debug.rs` 中已有的 `todo!("debug: ...")` 必须被替换为完整实现，不得残留。

---

## 四、 本次开发的硬防崩溃约束

- **1. 命令解析防空**: `Command::parse()` 对空白/空输入返回 `Err`，由 `read_command()` 捕获后静默忽略（不打印错误、不终止循环）。
- **2. 探针操作失败不崩溃**: 所有 `backend.*` 调用失败时以 `println!("[ERROR] ...")` 输出并 `continue`，不得 `unwrap()` 或 `panic!()`。
- **3. Ctrl+C 安全退出**: `rustyline::Editor::readline()` 在 Ctrl+C 时返回 `Err(ReadlineError::Interrupted)`，应优雅转换为 `Command::Quit`，不得 panic。

---

## 五、 本次规范验收评估核对

任何编码结果在被标记为完成提交至 Pull Request 前，执行 Agent 必须完成以下自检：

- [ ] **红线 1-6 检查**: `git diff --stat` 确认只修改了 `src/session.rs` 和 `src/cli/debug.rs`，未触碰 `src/probe/mod.rs`、`src/probe/probe_rs.rs`、`src/main.rs`（main.rs 的 Debug 路由）、`Cargo.toml`。
- [ ] **红线 7 检查**: `src/session.rs` 中 `fn new(chip_name: String)` 仍然存在，标有 `#[deprecated]`。
- [ ] **红线 8 检查**: `Command::parse("")`、`Command::parse("  ")`、`Command::parse("abc")`、`Command::parse("break")`（参数缺失）、`Command::parse("break abc")`（无效地址）全部在单元测试中覆盖且不 panic。
- [ ] **红线 10 检查**: `DebugArgs` 中 `json` / `break_at` / `watch_targets` / `continue_` / `halt_on_start` / `sampling_interval` / `serial_port` / `enable_flash_bp` / `no_flash` / `config` / `verify` / `backend` 字段在 `handle()` 中**没有被读取值**（只存在于参数传入的 `DebugArgs` 中，但代码不引用它们）。
- [ ] **红线 11 检查**: `DebugRepl::run()` 主循环中每个命令执行前有 `cmd.valid_states()` 检查。
- [ ] **零残留**: `src/cli/debug.rs` 中无 `todo!()` 残留。
- [ ] **测试 100%**: `cargo test -- --skip test_attach_without_hardware` 全部通过。
- [ ] **格式合规**: `cargo fmt --all -- --check` 零差异。
