# spec.md — OpenOCD 兜底烧录后端红线契约

> ⓘ 本文是 AI 代理在开发时必须严格遵守的不可触及红线。违例自动阻断。

---

## 一、 架构红线

- [红线1] **DebugProbe trait 签名冻结**: `src/probe/mod.rs` 中 trait 的方法签名不得因本次需求而修改（包括新增、删除、重命名、改参数）。
- [红线2] **probe-rs backend 冻结**: `src/probe/probe_rs.rs` 中的 `attach/detach/flash/halt/resume/step/breakpoint/mem/regs` 实现不得修改。
- [红线3] **OpenOCD/log/buffer/session 模块范围限定**: 仅允许修改 `src/probe/openocd.rs`。`src/log/`、`src/buffer/`、`src/session/` 禁止触碰。
- [红线4] **Cargo.toml 无新增依赖**: 不得新增任何外部 crate。
- [红线5] **CLI Commands 枚举完整保留**: `Init`/`Flash`/`Clean`/`Debug` 四个变体不可删除或重命名。`Flash` 变体仅允许追加 `--backend` 和 `--openocd-cfg` 字段。
- [红线6] **已有测试不可降级**: 已有 13 个测试（10 probe-rs + 3 flash）不得因新代码而修改或失败。

## 二、 实现红线

- [红线7] **`create_backend()` 必须唯一**: 后端选择逻辑封装在 `create_backend()` 函数内，不得在 `handle()` 中直接散落 if-else。
- [红线8] **`OpenOcdBackend` 已实现方法不可 panic**: `attach/flash/resume/detach` 必须使用 `anyhow::Result` 返回错误，不得使用 `unwrap()`、`expect()`、`todo!()`、`panic!()`。
- [红线9] **Standalone flash 不依赖 debug session**: `OpenOcdBackend` 的 flash 实现必须在进程内完成完整生命周期（spawn → flash → exit/kill），不与 `debug` 子命令共享状态。
- [红线10] **Drop guard 防僵尸**: `OpenOcdBackend` 必须实现 `Drop` trait，确保 `mcu-bridge` 进程异常退出时 OpenOCD 子进程被 kill。

## 三、 风格红线

- [红线11] **禁止通配导入**: 测试模块 `mod tests` 不得使用 `use super::*`，必须显式导入每个使用项。
- [红线12] **use 导入顺序**: `std` → 第三方 crate → `crate` 内部模块，每组之间空一行。
