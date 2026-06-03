# task.md - 精细化函数级开发任务清单与执行足迹

> ⓘ 本文件是实现特性的 AI 代理的核心执行手记。你必须保持在任意时刻**只能有 1 个任务处于 [in-progress] 状态**；完成一步后，运行对应的验证命令并打勾，再推进下一步。

---

## 📌 当前总览
- **源需求文档**: [user_plan/p0-probe-cli/p0-probe-cli.md](user_plan/p0-probe-cli/p0-probe-cli.md)
- **最新更新日期**: 2026-06-03 (已归档)
- **整体进度状态**: `completed`

---

## 一、 probe-rs backend — 结构体字段重构 (ProbeRsBackend)

- [x] **Task 1.1: 为 ProbeRsBackend 添加真实字段**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)`
  - **实施计划**:
    1. 将 `pub struct ProbeRsBackend;` 改为带字段的结构体：
       - `session: Option<probe_rs::Session>` — attach 后持有，detach 时释放
       - `active_core: usize` — 当前活跃核
       - `next_bp_id: BpId` — 断点 ID 计数器
       - `bp_map: HashMap<u64, BpId>` — 地址 → ID 映射
       - `next_wp_id: WpId` — watchpoint ID 计数器
    2. 实现 `ProbeRsBackend::new()` 和 `Default::default()`
    3. 添加私有辅助方法 `fn get_core(&mut self, core: Option<usize>) -> anyhow::Result<&mut probe_rs::Core<'_>>`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

---

## 二、 probe-rs backend — 12 个方法实现

- [x] **Task 2.1: 实现 attach + detach**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)`
  - **实施计划**:
    - `attach`: 用 `TargetSelector::Unspecified(chip.name)` + `SessionConfig::default()` → `Session::auto_attach()` → 存入 `self.session`
    - `detach`: `self.session = None`，清空 `bp_map`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 2.2: 实现 flash**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)`
  - **实施计划**:
    - 调用 `probe_rs::flashing::download_file(session, elf, Format::Elf(ElfOptions::default()))`
    - 错误用 `anyhow::anyhow!("flash failed: {e}")` 包装
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 2.3: 实现 halt + resume + step**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)`
  - **实施计划**:
    - `halt`: `core.halt(Duration::from_millis(500))`
    - `resume`: `core.run()`
    - `step`: `core.step()`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 2.4: 实现 core_count + active_core**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)`
  - **实施计划**:
    - `core_count`: `self.session.as_ref().map(|s| s.cores().len()).unwrap_or(0)`
    - `active_core`: 返回 `self.active_core`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 2.5: 实现 set_breakpoint + clear_breakpoint**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)`
  - **实施计划**:
    - `set_breakpoint`: `core.set_hw_breakpoint(addr as u64)` → 分配 ID → 存入 `bp_map`
    - `clear_breakpoint`: 从 `bp_map` 反查地址 → `core.clear_hw_breakpoint(addr)` → 从 `bp_map` 移除
    - 注意 clear 时避免借用冲突（先取地址、再获取 core）
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 2.6: 实现 read_mem + write_mem**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)`
  - **实施计划**:
    - `read_mem(addr, len, core)`: 从 addr 开始逐 4 字节 `read_word_32` → `to_le_bytes` → push 到 Vec → truncate(len)
    - `write_mem(addr, data, core)`: data.chunks(4) → 组装 u32 → `write_word_32`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 2.7: 实现 read_regs**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)`
  - **实施计划**:
    - `core.registers().core_registers()` 遍历
    - 对每个 reg 调用 `core.read_core_reg(reg.id())` → 转 u64 → 插入 HashMap
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 2.8: P1 预留方法 — is_connected/try_recover/set_watchpoint/clear_watchpoint/is_halted**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)`
  - **实施计划**:
    - `is_connected` → 返回 `self.session.is_some()`（P0 退化实现）
    - `try_recover` → `anyhow::bail!("P1: probe recovery not yet implemented")`
    - `set_watchpoint` → `anyhow::bail!("P1: watchpoint not yet implemented")`
    - `clear_watchpoint` → 同上
    - `is_halted` → 返回 `false`（P0 退化实现）
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

---

## 三、 mock 测试

- [x] **Task 3.1: 编写 mock 单元测试**
  - **受影响文件**: `[src/probe/probe_rs.rs](src/probe/probe_rs.rs)` — `#[cfg(test)]` section
  - **测试函数清单**:
    1. `test_backend_creation` — 验证初始状态（未连接、core_count=0）
    2. `test_attach_without_hardware` — 无硬件时 attach 返回 Err（不 panic）
    3. `test_detach_is_idempotent` — 未连接下两次 detach 均成功
    4. `test_halt_without_attach_returns_error` — 未连接下 halt 返回 Err 且含 "not attached"
    5. `test_resume_without_attach_returns_error` — 同上
    6. `test_step_without_attach_returns_error` — 同上
    7. `test_read_mem_without_attach_returns_error` — 同上
    8. `test_default_creates_empty_backend` — Default::default() 的 core_count=0
    9. `test_p1_methods_return_error_not_panic` — P1 方法返回 Err 不 panic
  - **本地验证命令**: `cargo test --lib -- probe_rs`
  - **当前状态**: `completed`

---

## 四、 CLI init 子命令

- [x] **Task 4.1: 实现 init 子命令**
  - **受影响文件**: `[src/cli/init.rs](src/cli/init.rs)`
  - **实施计划**:
    1. 硬编码 `get_chip_template(name: &str) -> Option<ChipConfig>` 函数，至少含 STM32F407VG
    2. `handle()` 流程：查模板 → 构建 `AppConfig`（填入用户指定的 debugger/interface 参数）→ `toml::to_string_pretty` 渲染 → 创建 `.debugger/` 目录 → 写入 `.debugger/chip.toml`
    3. 输出 `[INFO] config written to .debugger/chip.toml`
  - **本地验证命令**: `cargo run -- init --chip STM32F407VG --debugger stlink-v2 --interface swd && cat .debugger/chip.toml`
  - **当前状态**: `completed`

---

## 五、 CLI clean 子命令

- [x] **Task 5.1: 实现 clean 子命令**
  - **受影响文件**: `[src/cli/clean.rs](src/cli/clean.rs)`
  - **实施计划**:
    1. `handle()` 流程：`dirs::home_dir()` → `.mcu_bridge/` 路径
    2. `--all` → `remove_dir_all` 整个目录
    3. `--older-than` → 解析时间字符串（后缀 s/m/h/d/w）→ 比较 dir 修改时间 → 逐个删除
    4. 无参数 → 计算当前项目 hash（用 `std::env::current_dir()` hash）→ 删除对应子目录
    5. 打印清理计数
  - **本地验证命令**: `cargo run -- clean --all`
  - **当前状态**: `completed`

---

## 六、 CLI flash 子命令 (dry-run)

- [x] **Task 6.1: 实现 flash dry-run**
  - **受影响文件**: `[src/cli/flash.rs](src/cli/flash.rs)`
  - **实施计划**:
    1. 校验 `args.elf` 存在性：`args.elf.exists()` 否则 `anyhow::bail!(...)`
    2. 输出诊断信息：`flash: ELF={}, chip={}, verify={}`
    3. 返回 `Ok(())`
  - **本地验证命令**: `cargo run -- flash --elf Cargo.toml`
  - **当前状态**: `completed`

---

## 七、 全量验收

- [x] **Task 7.1: 移除 main.rs 全局 allow + cargo check 零 warning**
  - **受影响文件**: `[src/main.rs](src/main.rs)`
  - **实施计划**: 移除 `#![allow(dead_code, unused_imports)]`；如有剩余 P2 模块需要局部 `#[allow(dead_code)]`
  - **本地验证命令**: `cargo check 2>&1 | grep -c warning` → 0 (或仅 openocd/rtt 等 P2 模块)
  - **当前状态**: `completed`

- [x] **Task 7.2: cargo fmt**
  - **执行命令**: `cargo fmt --all -- --check`
  - **当前状态**: `completed`

- [x] **Task 7.3: cargo test**
  - **执行命令**: `cargo test --lib`
  - **当前状态**: `completed`

- [x] **Task 7.4: cargo run -- --help + init/clean/flash 端到端**
  - **执行命令**: `cargo run -- --help` → `cargo run -- init --chip STM32F407VG` → `cargo run -- flash --elf Cargo.toml`
  - **当前状态**: `completed`
