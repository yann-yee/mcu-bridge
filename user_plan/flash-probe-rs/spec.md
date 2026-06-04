# spec.md - 编码过程沙盒保险箱与红线契约

> ⓘ 本文件是本次代码重构的「保险圈与硬约束控制中心」。后续 Agent 在进行编码时必须 100% 保持在这些红线圈内。发生任何破坏或规避以下约定的改动，均不可被提交或合并进主分支。

---

## 一、 控制沙盒与严禁篡改红线 (Strict Boundary / Do Not Touch)

绝对禁止 Agent 修改或影响的区域与架构规则：

- **红线 1 [DebugProbe trait 签名冻结]**:
  - `[src/probe/mod.rs](src/probe/mod.rs)` 中的 `DebugProbe` trait 17 个方法签名已成熟稳定。本次编码**严禁修改**任何方法签名（参数类型、返回值类型、方法名），包括 `attach/detach/flash`。

- **红线 2 [probe-rs backend 实现冻结]**:
  - `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)` 中 `ProbeRsBackend` 的 `flash()`、`attach()`、`detach()` 方法已有真实实现且通过测试。本次编码**严禁修改**这些方法的现有逻辑。只从 CLI 层调用它们。

- **红线 3 [OpenOCD / log / buffer / session 模块不可触碰]**:
  - `[src/probe/openocd.rs](src/probe/openocd.rs)`、`[src/log/](src/log/)`、`[src/buffer/](src/buffer/)`、`[src/session.rs](src/session.rs)` 属于 P2 或已完成模块，本次**严禁修改**任何内容。

- **红线 4 [Cargo.toml 依赖不变]**:
  - 当前依赖已覆盖本次需求。**严禁新增任何外部 crate**。所有功能仅使用 `probe-rs`、`anyhow`、`toml`、`serde`、`log` 等已有依赖。

- **红线 5 [CLI 子命令枚举结构完整保留]**:
  - `[src/cli/mod.rs](src/cli/mod.rs)` 中 `Commands` 枚举的 4 个变体（`Init`/`Flash`/`Clean`/`Debug`）均不可删除或重命名。仅允许在 `Commands::Flash` 中追加 `run: bool` 字段。

- **红线 6 [existing tests 100% pass retention]**:
  - `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)` 中的 10 个单元测试必须在本次变更后仍然 100% 通过。**严禁**为迁就新代码而修改已有的测试用例。

---

## 二、 编码设计规范（代码风格偏好对齐）

本次功能开发需要严格执行的代码品质契约：

- **1. 错误处理链式统一**: 所有 `anyhow::Result` 的 error 转换使用 `.map_err(|e| anyhow::anyhow!("context: {e}"))` 格式，保持与现有代码一致。禁止使用 `unwrap()`、`expect()`。
- **2. 进度输出约定**: 诊断信息/进度走 `eprintln!`，最终结果走 `println!`。JSON-Lines 协议未来将占用 stdout，stderr 必须保持清晰可分离。
- **3. `use` 导入顺序**: 严格遵循 `std::` → 第三方 crate → `crate::` 的顺序，每组之间空一行。禁止 `use super::*` 通配导入。
- **4. 测试命名**: 所有新增测试函数以 `test_flash_` 前缀命名，描述预期行为。

---

## 三、 本次开发的硬防崩溃约束

- 1. **函数传参防空保护**: `resolve_chip_config()` 中，文件读取和 TOML 解析失败必须通过 `?` 或 `map_err` 传播，绝不 panic。
- 2. **Session 空值防护**: 由于 `handle()` 内部创建全新 `ProbeRsBackend`，不存在 session 为空的问题。但 `backend.flash()` 调用到 `probe-rs` API 时如果 attach 失败，必须在 `attach()` 层就 bail。
- 3. **ELF 文件存在性前置校验**: 烧录前必须在 `handle()` 入口处（`attach` 之前）校验 `args.elf.exists()`，避免无需连接探针就失败的高成本操作。

---

## 四、 本次规范验收评估核对

任何编码结果在被标记为完成提交至 Pull Request 前，执行 Agent 必须完成以下自检：

- [ ] 没有任何 `// TODO`、`todo!()` 残留于 `[src/cli/flash.rs](src/cli/flash.rs)`。
- [ ] 确实在本地运行了 `cargo check`，不存在任何静态或类型报错。
- [ ] 运行 `cargo fmt --all -- --check`，无格式差异。
- [ ] 运行 `cargo test --lib`，全部测试通过。
- [ ] 运行 `cargo clippy --all-targets --all-features -- -D warnings`（若 clippy-driver 可用），无 warning。
- [ ] 运行 `git diff` 核实改动面积，100% 契合 [task.md](task.md) 规定的 4 个文件（`src/cli/mod.rs`、`src/cli/flash.rs`、`src/cli/init.rs`、`src/main.rs`），无出轨改动。
