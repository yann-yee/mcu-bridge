# spec.md — Debug Round 2 编码红线契约

> ⓘ 本文件是本次编码的"保险圈与硬约束控制中心"。后续 Agent 在进行编码时必须 100% 保持在这些红线圈内。发生任何破坏或规避以下约定的改动，均不可被提交或合并进主分支。

---

## 一、 控制沙盒与严禁篡改红线 (Strict Boundary / Do Not Touch)

绝对禁止 Agent 修改或影响的区域与架构规则：

- **红线 1 [DebugProbe trait 签名冻结]**: `[src/probe/mod.rs](../../src/probe/mod.rs)` 中的 `DebugProbe` trait 不得因本次需求而修改（包括新增、删除、重命名、改参数）。所有 18 个方法签名保持原样。

- **红线 2 [probe-rs 与 openocd 后端冻结]**: `[src/probe/probe_rs.rs](../../src/probe/probe_rs.rs)` 和 `[src/probe/openocd.rs](../../src/probe/openocd.rs)` 实现不得修改。所有已实现的方法保持原样。

- **红线 3 [main.rs CLI 路由冻结]**: `[src/main.rs](../../src/main.rs)` 中 `Commands::Debug` 的 15 字段解构+传入 `DebugArgs` 的代码已经存在，**不得修改**。

- **红线 4 [Cargo.toml 无新增依赖]**: 不得新增任何外部 crate。`serde_json` 已在依赖中（版本 1）。`serde` 是 serde_json 的传递依赖也可用。

- **红线 5 [CLI Commands 枚举完整保留]**: `Init` / `Flash` / `Clean` / `Debug` 四个变体不可删除或重命名。

- **红线 6 [已有测试全量保护]**: 已有的 46 个测试不得因新代码被修改或删除。新代码必须全部通过。

---

## 二、 实现红线 (Implementation Constraints)

- **红线 7 [`Session::new()` 保留]**: 已有 `Session::new(chip_name: String)` 构造函数必须保留，**只能追加 `#[deprecated]` 属性**，不得删除或修改签名。`Session::default()` 不变。

- **红线 8 [JsonSession stdio 使用限制]**: `JsonSession` 必须使用 `std::io::stdin().lines()` 和 `println!` 进行 I/O，不得使用 `rustyline`（Human REPL 专用），不得使用其他 I/O crate。

- **红线 9 [JSON 解析不可 panic]**: 所有 JSON 解析必须使用 `serde_json::from_str` 的 `Result` 路径，不得 `unwrap()` 或 `expect()`。解析失败时返回 `{"id":null,"status":"error","error":{"code":"E_PARAM","message":"..."}}`。

- **红线 10 [事件检测不可阻塞]**: `try_check_halted()` 中的 `is_halted()` 调用必须容忍失败（探针断连场景），不得 `unwrap()`。当前 `ProbeRsBackend::is_halted()` 返回 false（stub），事件检测逻辑即使不触发也必须能通过。

- **红线 11 [状态守卫强制]**: JSON-Lines 模式中每条命令执行前必须检查 `valid_states()`。状态不合法时返回 `E_STATE` 响应，不得执行命令。

- **红线 12 [Schema 命令手写元数据]**: schema 返回的命令元数据使用常量/函数硬编码（不是代码反射生成）。手写一个 `fn command_metadata() -> Vec<CommandMeta>` 包含全部 10 个命令。

- **红线 13 [portable-json 序列化]**: 所有写入 stdout 的 JSON 必须使用 `serde_json::to_string`（紧凑格式，无多余空白），后接 `\n`。不得使用 `serde_json::to_string_pretty`。每条 stdout 输出必须严格是一行一个完整 JSON 对象。

- **红线 14 [flash 流程复用 Session.backend]**: debug session 的烧录通过 `session.backend.flash()` 直接调用（同一 session），不得创建临时后端。`session.backend` 字段已经是 `pub`，可直接访问。

- **红线 15 [无残留 todo!()]**: `src/cli/debug.rs` 中已有的 `// TODO: Round 2 — 处理以下参数` 注释块必须被替换为完整实现，不得残留。

---

## 三、 编码设计规范 (Coding Style)

- **1. 错误消息语言**: 用户可见的错误消息使用英文（与现有 `debug.rs` / `flash.rs` 风格一致）。
- **2. 测试模块结构**: `#[cfg(test)]` 包裹的 `mod tests` 放在文件末尾。每个 `use` 导入显式指定，禁止 `use super::*`。
- **3. JSON 字段命名**: Rust 结构体字段使用 snake_case（serde 默认），JSON 输出也是 snake_case。不在 serde 上使用 `#[serde(rename)]` 除非协议强制要求。
- **4. Schema 命令元数据格式**: 每条命令的描述、参数模式、valid_states 使用 `Vec<CommandMeta>` 常量，定义在 `impl JsonSession` 的关联函数中。
- **5. 事件推送时机**: 仅在 session state 从 Running 切换到 Halted 时推送一次 `halted` 事件。避免重复推送（使用状态比较而非 is_halted 的绝对值）。

---

## 四、 本次开发的硬防崩溃约束

- **1. 命令解析防空**: `JsonSession::read_request()` 对空行/空白输入返回 None（静默忽略），不得 panic。
- **2. 探针操作失败不崩溃**: 所有 `backend.*` 调用失败时以 JSON 错误响应返回，不得 `unwrap()` 或 `panic!()`。
- **3. Ctrl+C 安全退出**: `JsonSession::run()` 在 stdin 读取到 Err 时（如 Ctrl+C 导致 BrokenPipe），应优雅 break 退出循环后 detach，不得 panic。

---

## 五、 本次规范验收评估核对

任何编码结果在被标记为完成前，必须完成以下自检：

- [ ] **红线 1-6 检查**: `git diff --stat` 确认只修改了 `src/session.rs`、`src/cli/debug.rs`、`src/cli/mod.rs` 和新增了 `src/cli/json_session.rs`。未触碰 `src/probe/mod.rs`、`src/probe/probe_rs.rs`、`src/probe/openocd.rs`、`src/main.rs`、`Cargo.toml`。
- [ ] **红线 7 检查**: `Session::new()` 仍然存在，标有 `#[deprecated]`。
- [ ] **红线 9-11 检查**: 所有 `serde_json::from_str` / `is_halted` / `valid_states` 调用都正确处理 Err 路径，无 `unwrap()` 残留。
- [ ] **红线 15 检查**: `handle()` 中无 `// TODO: Round 2` 或 `todo!()` 残留。
- [ ] **测试 100%**: `cargo test -- --skip test_attach_without_hardware` 全部通过。
- [ ] **格式合规**: `cargo fmt --all -- --check` 零差异。
- [ ] **Clippy 合规**: `cargo clippy --all-targets --all-features -- -D warnings` 零警告。
