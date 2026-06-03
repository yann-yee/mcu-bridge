# spec.md - 编码过程沙盒保险箱与红线契约

> ⓘ 本文件是本次代码重构的「保险圈与硬约束控制中心」。后续 Agent 在进行编码时必须 100% 保持在这些红线圈内。发生任何破坏或规避以下约定的改动，均不可被提交或合并进主分支。

---

## 一、 控制沙盒与严禁篡改红线 (Strict Boundary / Do Not Touch)

绝对禁止 Agent 修改或影响的区域与架构规则：

- **红线 1 [DebugProbe trait 签名冻结]**: 
  - `src/probe/mod.rs` 中的 `DebugProbe` trait 17 个方法签名已与设计文档 §3.1 对齐并经 Grill 三问确认。本次编码严禁修改任何方法签名（包括参数类型、返回值类型、方法名）。probe-rs 0.31 的 API 只能适配已有签名，不做反向修改。

- **红线 2 [Cargo.toml 依赖不变]**: 
  - 当前 10 个依赖已覆盖本次所有需求，严禁新增任何外部 crate。probe-rs 0.31 的公开 API（`Session::auto_attach`、`Core::halt/run/step/read_word_32` 等）已探明，无需额外依赖。

- **红线 3 [openocd.rs / log/* / buffer/* untouched]**: 
  - `src/probe/openocd.rs` 的 `todo!()` 骨架属于 P2，本次不可触碰。`src/log/*`、`src/buffer/*`、`src/session.rs` 属于 P1/P2，同样不可改。

- **红线 4 [CLI 子命令参数结构不变]**: 
  - `src/cli/mod.rs` 中 `Commands` 枚举的 4 个变体及其 `#[arg]` 参数定义不可增删。`init`/`clean`/`flash` 的 `Args` struct 字段不可变。

---

## 二、 编码设计规范（代码风格偏好对齐）

- **1. 错误处理**: 所有 `DebugProbe` 方法内部调用 probe-rs API 时，用 `.map_err(|e| anyhow::anyhow!("context: {e}"))` 统一转换。P1 预留方法用 `anyhow::bail!("P1: ...")`。
- **2. `use` 顺序**: 严格 `std::` → 第三方 → `crate::`，每组间空一行。
- **3. 测试命名**: `#[cfg(test)] mod tests` 内每个测试函数以 `test_` 前缀命名，描述预期行为。

---

## 三、 本次开发的硬防崩溃约束

- 1. **probe-rs API 空值防护**: `ProbeRsBackend.session` 为 `Option<Session>`，每次操作前 `.as_mut().ok_or_else(|| anyhow::anyhow!("not attached"))?`。
- 2. **Flash 烧录无硬件防护**: `download_file` 直接调用 probe-rs API，错误用 `anyhow` 链式包装并附带原始错误信息。

---

## 四、 本次规范验收评估核对

- [ ] 没有任何 `// TODO`、`todo!()` 残留于 `src/probe/probe_rs.rs` 和 `src/cli/{init,clean,flash}.rs`
- [ ] `cargo fmt --all -- --check` 通过
- [ ] `cargo check` 零 warning（或仅 openocd/P2 模块有 `dead_code`）
- [ ] `cargo test --lib` 全部绿色
