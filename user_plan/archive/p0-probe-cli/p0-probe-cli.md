# 需求规格说明书：P0 — DebugProbe backend + CLI 子命令实现

> ⓘ 本文档已经过 "Understanding (Grill Me)" 技能对齐，记录了逻辑共识。本文件已于 [user_plan/p0-probe-cli/p0-probe-cli.md](user_plan/p0-probe-cli/p0-probe-cli.md) 归档。实现该功能的后续 Agent 必须全盘以此文档为验收终点。

---

## 一、 功能概览与基本愿景 (User Story & Goal)

- **新功能背景**: `mcu-bridge` 骨架已就位（18 个 .rs 文件、Cargo.toml、CI）。P0 是第一个有业务逻辑的阶段——完成 probe-rs backend 的 12 个核心方法 + CLI 三个子命令的 Happy Path。这是项目从 "空壳" 到 "可编译可运行可测试" 的关键一步。
- **用户故事 (User Story)**: 作为一名开发者/Agent，我想要 `cargo run -- init --chip STM32F407VG` 能生成出正确的 `.debugger/chip.toml` 配置文件，且 `cargo test` 能证明 probe-rs backend 的 12 个方法正确封装了 probe-rs API，以便 P1 阶段可以直接接入 debug REPL 循环。
- **关联已有的技术链**:
  - `src/probe/mod.rs` — `DebugProbe` trait 17 方法骨架（本次填充 12 个）
  - `src/probe/probe_rs.rs` — 当前所有方法体为 `todo!()`（本次全部替换）
  - `src/cli/init.rs` `src/cli/clean.rs` `src/cli/flash.rs` — 当前 `handle()` 为 `todo!()`
  - `src/config.rs` — `AppConfig` / `ChipConfig` 类型体系已就位
  - `src/main.rs` — 入口分发已就位，`#![allow(dead_code, unused_imports)]` 需移除

---

## 二、 极限对齐的业务流程 (Consensus Flow & Sequence)

> 记录烤问阶段彻底敲定的技术决策与方案选择。

### 1. 标准顺畅流 (Happy Path)

**probe-rs backend 实现**：
1. 读取 probe-rs 0.31 的 API 文档（`Session` / `Core` / `Probe` 的公开方法签名）
2. 在 `src/probe/probe_rs.rs` 中为 `ProbeRsBackend` 添加真实字段（`Option<Session>` 等）
3. 逐个实现 12 个方法：`attach`/`detach`/`flash`/`halt`/`resume`/`step`/`core_count`/`active_core`/`set_breakpoint`/`clear_breakpoint`/`read_mem`/`write_mem`/`read_regs`
4. 编写 mock 单元测试验证：参数转发正确、错误转换正确、类型适配正确
5. `cargo test --lib` 全部通过

**init 子命令**：
1. 硬编码至少一个芯片模板（STM32F407VG：Cortex-M4、Flash 1MB@0x08000000、RAM 128KB@0x20000000）
2. 用户 `cargo run -- init --chip STM32F407VG --debugger stlink-v2 --interface swd` → 自动填充模板 + 用户参数 → 渲染 TOML → 写入 `.debugger/chip.toml`
3. 输出成功信息到终端

**clean 子命令**：
1. 定位 `~/.mcu_bridge/`（通过 `dirs::home_dir()`）
2. `mcu-bridge clean`（无参数）→ 删除当前项目 hash 对应的会话目录
3. `mcu-bridge clean --all` → 删除整个 `~/.mcu_bridge/`
4. `mcu-bridge clean --older-than 7d` → 按时间过滤删除
5. 输出清理统计

**flash 子命令 (dry-run)**：
1. 解析 `--elf` 参数（文件存在性校验）
2. 输出诊断信息：`flash: ELF=<path>, chip=<name>, verify=<bool>`
3. 返回 `Ok(())`——实际烧录留 P1

### 2. 异常与阻断流 (Exception Handlings)

- **probe-rs API 版本不匹配**: 如果 probe-rs 0.31 的方法签名与设计文档预期不同，以 `cargo doc` 实际 API 为准，更新 `DebugProbe` trait 签名同步适配
- **TOML 写入权限失败**: `init` 写入 `.debugger/chip.toml` 时目录不存在 → 自动创建 `.debugger/` 目录。写入无权限 → 报错 `E_INTERNAL`
- **缓存目录不存在**: `clean` 时 `~/.mcu_bridge/` 不存在 → 打印 "nothing to clean"
- **--elf 文件不存在**: `flash` 的 dry-run 阶段校验 `Path::exists()`，不存在则报错 `E_PARAM`
- **`#![allow(dead_code)]` 移除后暴露 warning**: 移除 `main.rs` 顶部的全局 allow 后，未使用的导入和结构体会产生 warning。本次实现后大部分 struct 会被使用，剩余的个别项加局部 `#[allow(dead_code)]` 而非全局

---

## 三、 烤问决策记录 (Grill Decisions)

本需求在 Understanding 阶段经历了三轮极限追问。以下为所有敲定的技术分歧点：

### 🔧 决策 1：P0 backend 方法范围 → 12 个方法（留 5 个给 P1）

- **实现**: `attach`/`detach`/`flash`/`halt`/`resume`/`step`/`core_count`/`active_core`/`set_breakpoint`/`clear_breakpoint`/`read_mem`/`write_mem`/`read_regs`
- **留 P1**: `is_connected`/`try_recover`/`set_watchpoint`/`clear_watchpoint`/`is_halted`（自恢复和数据观测属于后续阶段）
- **理由**: 断点和内存读写是最基本的调试原语，P0 如果不包含它们，P1 的 debug REPL 就没有可用的原语集合。12 个方法中大部分 probe-rs API 直接一一对应，工作量非线性增长。
- **否定方案**: 严格 5 方法（P0 交付后不可用）、全 17 方法（一次到位但 P0 膨胀）

### 🔧 决策 2：测试策略 → 纯 mock 单元测试（P0 不连硬件）

- **理由**: probe-rs API 本身已被广泛测试，我们只测自己的胶水代码（参数转发、错误转换、类型适配）。用 mock 替代真实 Session 完全够。AGENTS.md "无测试不交付" 不能破。真实硬件集成测试留 P1 Docker CI。
- **否定方案**: P0 手动硬件集成（拖慢 build-test 循环）、跳过测试（违反 AGENTS.md）

### 🔧 决策 3：CLI 子命令深度 → init + clean 全通，flash dry-run

- **理由**: `init`（芯片模板+TOML 渲染）和 `clean`（文件系统操作）不依赖 probe-rs backend，P0 独立交付。`flash` 依赖 `DebugProbe` trait 实现，串行依赖决定了 P0 只能做到 dry-run 骨架。
- **否定方案**: 三个全通（flash 等 backend 完）、三个全骨架（太保守）

---

## 四、 技术契约定义 (Technical Contract)

### 4.1 probe-rs backend 方法映射

probe-rs 0.31 的 `Session` / `Core` API → `DebugProbe` trait 方法：

| DebugProbe 方法 | probe-rs API | 关键步骤 |
|-----------------|-------------|---------|
| `attach` | `Session::auto_attach(chip_name)` → 创建 Session | 解析芯片名、选择 probe、连接目标 |
| `detach` | 无需显式操作 | drop Session 即可 |
| `flash` | `Flasher::new(session, ...).program(elf)` | ELF 解析 → 擦除 → 编程 → 可选校验 |
| `halt` | `core.halt(Duration::from_millis(500))` | core = session.core(core_idx) |
| `resume` | `core.run()` | |
| `step` | `core.step()` | |
| `core_count` | `session.cores().len()` | |
| `active_core` | 返回 `ProbeRsBackend` 内部字段 | |
| `set_breakpoint` | `core.set_hw_breakpoint(addr)` | 返回 `BpId` |
| `clear_breakpoint` | `core.clear_hw_breakpoint(id)` | |
| `read_mem` | `core.read_32(addr)` 循环 | 组装 `Vec<u8>` |
| `write_mem` | `core.write_32(addr, word)` 循环 | |
| `read_regs` | `core.registers()` → HashMap | |

### 4.2 init 子命令芯片模板

硬编码至少一个完整模板（后续 P2 芯片模板库可从此扩展）：

```
STM32F407VG:
  architecture = "cortex-m4"
  flash_base = 0x08000000, flash_size = 0x100000 (1MB)
  ram_base = 0x20000000, ram_size = 0x20000 (128KB)
```

### 4.3 clean 子命令时间解析

`--older-than Ns|Nm|Nh|Nd|Nw` 格式：
- `N` 为数字，后缀 `d`=天 `h`=小时 `m`=分钟 `s`=秒
- 解析后计算阈值时间戳，比较会话目录的修改时间

---

## 五、 验收断言与 Harness 测试指标 (Definition of Done)

> 绝对禁止空洞通过。以下每条都必须通过命令验证。

- [ ] **1. probe-rs backend 编译断言**: 移除 `ProbeRsBackend` 所有 `todo!()` 方法体并替换为真实实现后，`cargo check` 零 error 零 warning。
- [ ] **2. mock 测试断言**: `cargo test --lib` 至少包含以下测试用例：
  - `test_attach_detach` — 验证 attach 正确解析芯片名、detach 正确释放 session
  - `test_halt_resume_step` — 验证执行控制方法调用链正确（mock Core）
  - `test_breakpoint_set_clear` — 验证断点 ID 分配和清除
  - `test_read_write_mem` — 验证内存读写字节序和长度正确
  - `test_read_regs` — 验证寄存器快照 HashMap 键值格式
- [ ] **3. init Happy Path 断言**: `cargo run -- init --chip STM32F407VG --debugger stlink-v2 --interface swd` 生成 `.debugger/chip.toml`，文件内容含 `[chip]` / `[debugger]` / `[flash]` / `[serial]` / `[watch]` / `[recovery]` / `[flash_bp]` 全部 section。
- [ ] **4. clean Happy Path 断言**: `cargo run -- clean --all` 在 `~/.mcu_bridge/` 存在时删除所有子目录并打印清理计数；目录不存在时打印 "nothing to clean" 不报错。
- [ ] **5. flash dry-run 断言**: `cargo run -- flash --elf Cargo.toml` → 打印 `flash: ELF=Cargo.toml, ...` 诊断信息 → exit 0。`cargo run -- flash --elf nonexistent.elf` → 报错 exit ≠ 0。
- [ ] **6. 移除骨架期 allow 断言**: `src/main.rs` 的 `#![allow(dead_code, unused_imports)]` 已移除，`cargo check` 零 warning（或仅余局部 `#[allow(dead_code)]` 在 `openocd.rs` 等 P2 模块）。
- [ ] **7. 全量测试通过断言**: `cargo test --lib` 全部绿色，零 FAILED。
- [ ] **8. Lint 零容忍断言**: `cargo clippy --all-targets --all-features -- -D warnings` 通过（若 clippy-driver 不可用则手动检查代码无任何 `#[allow(clippy::xxx)]`）。
