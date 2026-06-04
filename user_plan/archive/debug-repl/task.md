# task.md — 精细化函数级开发任务清单与执行足迹

> ⓘ 本文件是实现特性的 AI 代理的核心执行手记。每一个步骤都精确写明了受影响文件、拟添加/修改的方法名称。你必须保持在任意时刻**只能有 1 个任务处于 [in-progress] 状态**；完成一步后，运行对应的验证命令并打勾，再推进下一步，保证开发路径 100% 可回溯。

---

## 📌 当前总览

- **源需求文档**: [user_plan/debug-repl/debug-repl.md](debug-repl.md)
- **最新更新日期**: 2026-06-05
- **整体进度状态**: `completed`

---

## 一、 开发准备 (Preparation)

- [x] **Task 1.1: 运行项目基础构建与测试校验**
  - **描述**: 证明在此次开发前，项目本地环境绝对完好，已有 17 个测试全部通过。
  - **本地执行检验命令**: `cargo test -- --skip test_attach_without_hardware`
  - **验证结果**: 17/17 测试通过 ✅

---

## 二、 Session 层扩展 (Data Layer)

- [x] **Task 2.1: 为 `Session` 结构体新增 `backend` 字段 + `attach()`/`detach()` 方法**
  - **受影响文件**: `[src/session.rs](src/session.rs)`
  - **函数/属性级实施计划**:
    1. 在 `use` 区块新增: `use crate::config::ChipConfig;` 和 `use crate::probe::DebugProbe;` 和 `use crate::probe::probe_rs::ProbeRsBackend;`
    2. 在 `pub struct Session` 字段末尾新增: `pub backend: Box<dyn DebugProbe>,`
    3. 新增 `impl Session` 方法:
       ```rust
       /// 连接探针并创建会话（初始状态 Halted）。
       pub fn attach(chip: &ChipConfig) -> anyhow::Result<Self> {
           let mut backend = ProbeRsBackend::new();
           backend.attach(chip)?;
           let core_count = backend.core_count();
           Ok(Self {
               state: SessionState::Halted,
               chip_name: chip.name.clone(),
               core_count,
               pc: None,
               bp_count: 0,
               watch_count: 0,
               backend: Box::new(backend),
           })
       }

       /// 安全 detach（drop 前调用）。
       pub fn detach(&mut self) -> anyhow::Result<()> {
           self.backend.detach()
       }
       ```
    4. `Session::new()` 保留不变（其他模块可能使用），加 `#[deprecated]` 属性。
  - **本地验证命令**: `cargo check`

---

## 三、 核心业务逻辑实现 (Backend Services)

### 3a. Command 枚举 + 解析

- [x] **Task 3.1: 新增 `Command` 枚举 + Display 实现**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 在 `use` 区块追加: `use crate::session::SessionState;` 和 `use std::fmt;`
    2. 新增 `pub enum Command`：
       ```rust
       #[derive(Debug, Clone, PartialEq, Eq)]
       pub enum Command {
           Halt,
           Resume,
           Step,
           Break { addr: u32 },
           Regs,
           Mem { addr: u32, len: u32 },
           Status,
           Help,
           Quit,
       }
       ```
    3. 为 `Command` 实现 `fmt::Display`，如 `Halt` → `"halt"`，`Break { addr }` → `"break 0x{addr:08x}"` 等。
  - **本地验证命令**: `cargo check`

- [x] **Task 3.2: 实现 `Command::parse()`**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 签名: `pub fn parse(input: &str) -> Result<Self, String>`
    2. 逻辑: `let trimmed = input.trim().to_lowercase();` → `trimmed.split_whitespace().collect::<Vec<_>>()` → match 命令名和参数个数
    3. 地址解析: 支持 `0x` 前缀十六进制和纯十进制，使用 `u32::from_str_radix`
    4. 长度解析: 仅十进制 `u32`
    5. 命令名不匹配 → `Err(format!("unknown command '{input}'"))`
    6. 参数个数不匹配 → `Err(format!("usage: break <addr>"))`
  - **本地验证命令**: `cargo test test_parse_`

- [x] **Task 3.3: 实现 `Command::valid_states()`**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 签名: `pub fn valid_states(&self) -> Option<&[SessionState]>`
    2. `Halt` → `Some(&[SessionState::Running])`
    3. `Resume | Step | Break | Regs | Mem` → `Some(&[SessionState::Halted])`
    4. `Status | Help | Quit` → `None`（任何状态均可）
  - **本地验证命令**: `cargo test test_*_valid_* test_*_invalid_*`

### 3b. DebugRepl 结构体 + 主循环

- [x] **Task 3.4: 新增 `DebugRepl` 结构体 + `new()` + `run()`**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 在 `use` 区块追加: `use rustyline::Editor;`、`use crate::session::Session;`
    2. 新增:
       ```rust
       pub struct DebugRepl {
           session: Session,
           rl: Editor<()>,
       }

       impl DebugRepl {
           pub fn new(session: Session) -> Self { ... }
           pub fn run(&mut self) -> anyhow::Result<()> { ... }
       }
       ```
    3. `run()` 主循环伪代码:
       ```
       loop {
           match self.read_command() {
               Some(Quit) => break,
               Some(cmd) => {
                   // 状态守卫
                   if let Some(states) = cmd.valid_states() {
                       if !states.contains(&self.session.state) {
                           println!("[ERROR] command '{cmd}' not valid in {:?} state", self.session.state);
                           continue;
                       }
                   }
                   // 执行
                   if let Err(e) = self.execute(cmd) {
                       println!("[ERROR] {e}");
                   }
               }
               None => continue,
           }
       }
       self.session.detach()
       ```
  - **本地验证命令**: `cargo build`

- [x] **Task 3.5: 实现 `read_command()`**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 签名: `fn read_command(&mut self) -> Option<Command>`
    2. 调用 `self.rl.readline("(mcu) > ")`
    3. 成功 → 追加到 history + `Command::parse` + 返回 `Some(cmd)`
    4. 空白行 → 返回 `None`
    5. Ctrl+C/Ctrl+D → 返回 `Some(Command::Quit)`
    6. parse 失败 → `println!("[ERROR] {err}")` → 返回 `None`
  - **本地验证命令**: `cargo build`

- [x] **Task 3.6: 实现 `execute()` dispatch**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 签名: `fn execute(&mut self, cmd: Command) -> anyhow::Result<()>`
    2. `match cmd` → 分派到对应 `cmd_*` 方法
  - **本地验证命令**: `cargo build`

### 3c. 各命令方法实现

- [x] **Task 3.7: 实现 `cmd_halt()` + `cmd_resume()` + `cmd_step()`**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. `cmd_halt()`: `self.session.backend.halt(None)?; self.session.state = SessionState::Halted; println!("[OK] target halted");`
    2. `cmd_resume()`: `self.session.backend.resume(None)?; self.session.state = SessionState::Running; println!("[OK] target running");`
    3. `cmd_step()`: `self.session.backend.step(None)?; let pc = ???; println!("[OK] stepped to 0x{pc:08x}");`
  - **注**: `cmd_step()` 中 PC 值需调用 `read_regs()` 提取 `pc` 或使用 `get_core()` 查询。为简化第一版可在 `step()` 后调用 `read_regs()` 并打印 `pc` 寄存器。
  - **本地验证命令**: `cargo build`

- [x] **Task 3.8: 实现 `cmd_break(addr)`**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 调用 `self.session.backend.set_breakpoint(addr, None)?`
    2. 成功 → `self.session.bp_count += 1; println!("[#{}] breakpoint at 0x{addr:08x}", self.session.bp_count - 1);`
  - **本地验证命令**: `cargo build`

- [x] **Task 3.9: 实现 `cmd_regs()`**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 调用 `self.session.backend.read_regs(None)?` → 得到 `HashMap<String, u64>`
    2. 遍历并打印: `println!("{name}\t0x{value:08x}")`，按自然顺序排序
    3. 更新 `self.session.pc`（若寄存器中包含 `pc` 字段）
  - **本地验证命令**: `cargo build`

- [x] **Task 3.10: 实现 `cmd_mem(addr, len)`**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 调用 `self.session.backend.read_mem(addr, len, None)?` → `Vec<u8>`
    2. 格式化输出：每行地址 + 16 字节十六进制 + ASCII 表示
    3. 示例:
       ```
       0x20000000  00 01 02 03 04 05 06 07  08 09 0a 0b 0c 0d 0e 0f  ................
       0x20000010  10 11 12 13 14 15 16 17  18 19 1a 1b 1c 1d 1e 1f  ................
       ```
  - **本地验证命令**: `cargo build`

- [x] **Task 3.11: 实现 `cmd_status()` + `print_help()`**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. `cmd_status()`:
       ```rust
       fn cmd_status(&self) {
           let pc_str = self.session.pc.map(|p| format!("0x{p:08x}")).unwrap_or("?".into());
           println!("state={:?}  chip={}  bp={}  pc={}  cores={}",
               self.session.state, self.session.chip_name,
               self.session.bp_count, pc_str, self.session.core_count);
       }
       ```
    2. `print_help()`: 打印所有命令的语法和描述表格（硬编码字符串）
  - **本地验证命令**: `cargo build`

### 3d. handle() 集成

- [x] **Task 3.12: 重写 `handle()` — ELF 校验 → 芯片解析 → 会话 → REPL**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 在文件顶部增加 `use`：`use crate::config::ChipConfig;`，`use crate::cli::init;`，`use std::path::Path;`
    2. 校验 ELF 存在：`if !args.elf.exists() { anyhow::bail!("ELF file not found: {}", args.elf.display()); }`
    3. 解析芯片配置：
       ```rust
       let chip_name = args.chip.clone().unwrap_or_else(|| {
           // 尝试读取 .debugger/chip.toml
           ...
       });
       let chip = init::get_chip_template(&chip_name)?;
       ```
    4. `Session::attach(&chip)?`
    5. `println!("[OK] attached to {}, {} core(s)", chip.name, session.core_count);`
    6. `let mut repl = DebugRepl::new(session); repl.run()?;`
    7. 全部完成后 `println!("[OK] debug session ended"); Ok(())`
  - **注意**: `--json` / `--break-at` / `--watch` / `--continue_` / `--halt-on-start` / `--sampling-interval` / `--serial-port` / `--enable-flash-bp` / `--no-flash` / `--config` / `--verify` / `--backend` 等参数在本轮 REPL 中**忽略**（只解析不实现）。
  - **本地验证命令**: `cargo build && cargo run -- debug --elf test_firmware/fw.elf --chip STM32F407VG --help`（仅验证 --help 输出，不启动硬件）

---

## 四、 测试层 (Test Coverage)

- [x] **Task 4.1: 新增命令解析单元测试（15 个）**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)` — 在 `#[cfg(test)] mod tests` 中新增
  - **测试函数清单**:
    - `test_parse_halt` — `"halt"` → `Ok(Halt)`
    - `test_parse_resume` — `"resume"` → `Ok(Resume)`
    - `test_parse_step` — `"step"` → `Ok(Step)`
    - `test_parse_break` — `"break 0x08000100"` → `Ok(Break { addr: 0x08000100 })`
    - `test_parse_break_decimal` — `"break 134219776"` → `Ok(Break { addr: 0x08000100 })`
    - `test_parse_break_no_addr` — `"break"` → `Err`
    - `test_parse_break_bad_addr` — `"break abc"` → `Err`
    - `test_parse_regs` — `"regs"` → `Ok(Regs)`
    - `test_parse_mem` — `"mem 0x20000000 16"` → `Ok(Mem { addr: 0x20000000, len: 16 })`
    - `test_parse_mem_missing_len` — `"mem 0x20000000"` → `Err`
    - `test_parse_status` — `"status"` → `Ok(Status)`
    - `test_parse_help` — `"help"` → `Ok(Help)`
    - `test_parse_quit_exit` — `"quit"` → `Ok(Quit)`, `"exit"` → `Ok(Quit)`
    - `test_parse_unknown` — `"xyz"` → `Err`
    - `test_parse_whitespace` — `""`, `"  "` → `Err`
  - **本地验证命令**: `cargo test test_parse_`

- [x] **Task 4.2: 新增状态守卫单元测试（9 个）**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)` — `#[cfg(test)] mod tests`
  - **测试函数清单**:
    - `test_halt_valid_in_running` — `Command::Halt.valid_states()` 在 `Running` 中应包含
    - `test_halt_invalid_in_halted` — `Command::Halt.valid_states()` 在 `Halted` 中应不包含
    - `test_resume_valid_in_halted` — `Resume` 在 `Halted` 中包含
    - `test_resume_invalid_in_running` — `Resume` 在 `Running` 中不包含
    - `test_step_valid_in_halted` — `Step` 在 `Halted` 中包含
    - `test_step_invalid_in_running` — `Step` 在 `Running` 中不包含
    - `test_break_valid_in_halted` — `Break` 在 `Halted` 中包含
    - `test_status_all_states` — `Status` 的 `valid_states()` 返回 `None`
    - `test_help_quit_all_states` — `Help` 和 `Quit` 返回 `None`
  - **本地验证命令**: `cargo test test_*_valid_* test_*_invalid_*`

- [x] **Task 4.3: 新增启动错误路径测试（3 个）**
  - **受影响文件**: `[src/cli/debug.rs](src/cli/debug.rs)` — `#[cfg(test)] mod tests`
  - **测试函数清单**:
    - `test_debug_elf_not_found` — `handle()` 传入不存在 ELF → `Err("ELF file not found")`
    - `test_debug_unknown_chip` — `handle()` 传入无效芯片名 → `Err("unknown chip")`
    - `test_debug_no_chip_no_config` — `--chip None` + 无 `.debugger/chip.toml` → `Err`
  - **本地验证命令**: `cargo test test_debug_`

---

## 五、 全局集成检验与 DoD 验证机制 (Whole Loop Verification)

- [x] **Task 5.1: 全量验证套件**
  - **描述**: 运行完整功能链，断言满足 DoD 指标。
  - **验证命令**:
    1. `cargo fmt --all -- --check` → 零差异 ✅
    2. `cargo test -- --skip test_attach_without_hardware` → 46/46 全部通过 ✅
    3. `cargo check` → 零 error ✅
    4. `cargo run -- debug --help` → 输出包含 `--elf`、`--chip` 等参数 ✅
  - **验证结果**: 全部通过 ✅
