# task.md — 精细化函数级开发任务清单与执行足迹

> ⓘ 本文件是实现特性的 AI 代理的核心执行手记。每一个步骤都精确写明了受影响文件、拟添加/修改的方法名称。你必须保持在任意时刻**只能有 1 个任务处于 [in-progress] 状态**；完成一步后，运行对应的验证命令并打勾，再推进下一步。

---

## 📌 当前总览

- **源需求文档**: [user_plan/flash-openocd-backend/flash-openocd-backend.md](flash-openocd-backend.md)
- **最新更新日期**: 2026-06-04
- **整体进度状态**: `completed`

---

## 一、 数据层扩展 (Data Layer)

- [x] **Task 1.1: `Commands::Flash` 变体新增 `--backend` 和 `--openocd-cfg` 参数**
  - `src/cli/mod.rs`: 追加 `#[arg(long)] backend: Option<String>` 和 `#[arg(long)] openocd_cfg: Option<String>`
  - **验证**: `cargo run -- flash --help` 显示 `--backend` + `--openocd-cfg` ✅

- [x] **Task 1.2: main.rs 模式匹配解构新字段**
  - `src/main.rs`: `Commands::Flash` destructure 追加 `backend`、`openocd_cfg`
  - **验证**: `cargo check` ✅

- [x] **Task 1.3: FlashArgs 结构体追加新字段**
  - `src/cli/flash.rs`: `FlashArgs` 追加 `backend: Option<String>`、`openocd_cfg: Option<String>`
  - **验证**: `cargo check` ✅

---

## 二、 OpenOCD Backend 实现 (Backend Implementation)

- [x] **Task 2.1: 实现 `OpenOcdBackend` 结构体 + 构造函数**
  - `src/probe/openocd.rs`: 添加 `process/telnet/cfg_path` 字段；`new(cfg_path)` 构造函数
  - **验证**: `cargo check` ✅

- [x] **Task 2.2: 实现 `attach()` — spawn OpenOCD + 轮询 TCP 6666**
  - `spawn_openocd()`: `Command::new("openocd")` 带 `-f`、tcl_port 6666、gdb_port disabled
  - `wait_for_telnet()`: 25 次 × 200ms 轮询，最多 5s
  - `DebugProbe::attach()`: 组合以上两步
  - **验证**: `cargo test test_openocd_attach_no_cfg` ✅

- [x] **Task 2.3: 实现 `flash()` + `resume()` — TCL `program` + `reset` 命令**
  - `tcl_command()`: 发送命令 → 逐行读取直到 `"> "` 提示符
  - `flash()`: 构建 `program <elf> verify`，检查响应不含 error/failed
  - `resume()`: 发送 `reset`，检查响应
  - **验证**: `cargo check` ✅

- [x] **Task 2.4: 实现 `detach()` + `Drop` guard — exit → wait → kill 保底**
  - `cleanup_process()`: send `exit` → 轮询 `try_wait()` 5s → `kill()` + `wait()` 保底
  - `Drop::drop()`: 调用 `cleanup_process()` 防僵尸
  - **验证**: `cargo check` ✅

- [x] **Task 2.5: 其他 DebugProbe 方法标记为 P2**
  - `halt/step/set_breakpoint/clear_breakpoint/set_watchpoint/...` → `anyhow::bail!("P2: ...")`
  - **验证**: `cargo check` ✅

---

## 三、 后端工厂与路由 (Backend Factory)

- [x] **Task 3.1: 实现 `create_backend()` 工厂函数**
  - `src/cli/flash.rs`: 根据 `--backend` > TOML `[debugger].backend` > `"probe-rs"` 选择后端
  - **验证**: `cargo test test_flash_backend_probe_rs_default test_flash_backend_openocd_no_cfg test_flash_backend_unknown` ✅

- [x] **Task 3.2: 重写 `handle()` 使用 Box\<dyn DebugProbe\>**
  - `src/cli/flash.rs`: `create_backend(args)` → `backend.attach()` → `flash()` → 可选 `resume()` → `detach()`
  - **验证**: `cargo test -- --skip test_attach_without_hardware` 17/17 ✅

---

## 四、 测试与红线验证 (Verification)

- [x] **Task 4.1: 新增 openocd 后端子模块测试**
  - `test_openocd_creation` — 初始状态校验
  - `test_openocd_attach_no_cfg` — 配置不存在校验
  - **验证**: `cargo test test_openocd_` ✅

- [x] **Task 4.2: 新增后端路由测试**
  - `test_flash_backend_probe_rs_default` — 默认 probe-rs
  - `test_flash_backend_openocd_no_cfg` — OpenOCD 无配置
  - `test_flash_backend_unknown` — 无效后端
  - **验证**: `cargo test test_flash_backend_` ✅

- [x] **Task 4.3: 全量验证套件**
  - `cargo fmt --all -- --check` → 零差异 ✅
  - `cargo test -- --skip test_attach_without_hardware` → 17/17 ✅
  - `cargo check` → 零 error ✅
  - `cargo run -- flash --help` → 包含 `--backend`、`--openocd-cfg` ✅

---

## 五、 文档落盘 (Documentation)

- [x] **Task 5.1: 三件套文档写入**
  - `flash-openocd-backend.md` — 需求规格说明书
  - `spec.md` — 红线契约
  - `task.md` — 本文件（执行足迹）
  - **验证**: 三文件完整存在于 `user_plan/flash-openocd-backend/` ✅

- [x] **Task 5.2: context.md 变更日志更新**
  - **验证**: 倒序追加归档登记 ✅
