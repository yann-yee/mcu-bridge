# 需求规格说明书：Debug REPL — Human 交互式调试会话（Round 1）

> ⓘ 本文档已经过 "Understanding (Grill Me)" 技能对齐，记录了 4 轮决策的完整共识。本文件已于 [user_plan/debug-repl/debug-repl.md](debug-repl.md) 归档。实现该特性的后续 Agent 必须全盘以此文档为验收终点。

---

## 一、 功能概览与基本愿景 (User Story & Goal)

- **新功能背景**: `mcu-bridge` 已有 flash 子命令（Standalone 烧录）和 debug 子命令骨架（`todo!()`）。当前无法交互式调试 MCU——用户/Agent 无法设断点、单步、读寄存器。本需求实现 **Round 1：Human REPL 交互调试**，覆盖最小可用调试闭环。
- **用户故事 (User Story)**: 作为一名嵌入式开发者，我想要在终端中运行 `mcu-bridge debug --elf fw.elf --chip STM32F407VG` 进入交互式 REPL，以便手动执行 halt / resume / step / break / regs / mem / status 等调试操作。
- **关联已有的技术链**:
  - `src/session.rs` — 已有 `Session` / `SessionState` 骨架，需扩展持有后端
  - `src/cli/debug.rs` — 当前 `todo!()`，需实现 `DebugRepl` + 命令 dispatch
  - `src/cli/mod.rs` — `Commands::Debug` 已定义全部 CLI 参数
  - `src/probe/probe_rs.rs` — halt/resume/step/breakpoint/mem/regs 已全部实现
  - `src/probe/mod.rs` — `DebugProbe` trait 已稳定
  - `rustyline` — 已在 Cargo.toml，版本 18

### 本轮不做（留待 Round 2+）

| 排除项 | 说明 |
|--------|------|
| ❌ JSON-Lines Agent 模式 (`--json`) | 参数已定义但不实现协议循环 |
| ❌ DebugBuffer 定时采样 + ring buffer | 变量观测留到后续特性 |
| ❌ LogChannel 集成 (RTT/UART/Semihosting) | 独立特性，不与 REPL 耦合 |
| ❌ `--watch` / `--break-at` / `--continue` / `--halt-on-start` | CLI 参数已定义但不实现启动逻辑 |
| ❌ `flash` 命令在 REPL 内部 | Standalone `mcu-bridge flash` 已独立实现 |
| ❌ OpenOCD backend | 后续特性 |
| ❌ DWARF 符号解析 | P3 特性 |

---

## 二、 极限对齐的业务流程 (Consensus Flow & Sequence)

### 共识决策树

| 决策节点 | 选定方案 | 替代方案否决理由 |
|---------|---------|----------------|
| Q1: 交付范围 | `[C]` 先 Human REPL，再 Agent 模式 | A(仅 8 命令无结构) 快速腐烂；B(全量 P1) 风险大、代码量 600-1000 行 |
| Q2: 命令集 | `[C]` 结构化 Command enum + 8 命令 + help | A(字符串匹配) 无法扩展参数；B(16 条) 超出最小可用集 |
| Q3: 架构模式 | `[B]` DebugRepl 结构体 + 方法拆分 | A(单函数) 200-300 行不可维护、不可测试 |
| Q4: 状态持有 | `[A]` Session 扩展持有 Box\<dyn DebugProbe\> | B(DebugRepl 全持) 使 session.rs 成死代码 |

### 1. 标准顺畅流 (Happy Path)

1. 用户执行 `mcu-bridge debug --elf fw.elf --chip STM32F407VG`
2. `handle()` 解析 CLI 参数，校验 ELF 存在，加载芯片配置
3. 创建 `ProbeRsBackend` → `backend.attach(&chip)` 连接探针
4. 创建 `Session { state: Halted, backend, chip_name, ... }`
5. 创建 `DebugRepl { session, rl }` → 进入交互主循环
6. 显示提示符 `(mcu) > `，等待用户输入
7. 用户输入 `break 0x08000100` → 设硬件断点 → 打印 `[#0] breakpoint at 0x08000100`
8. 用户输入 `resume` → 全速运行 → 状态切换为 Running
9. 断点命中 → 状态自动切换为 Halted → 打印 `[halted] breakpoint #0 at 0x08000100`
10. 用户输入 `regs` → 读取并打印寄存器快照（名称=值 每行一个）
11. 用户输入 `mem 0x20000000 16` → 读取 16 字节 → 十六进制 dump
12. 用户输入 `step` → 单步执行 → 打印当前 PC
13. 用户输入 `status` → 打印会话摘要（状态、芯片、断点数、PC）
14. 用户输入 `quit` → `backend.detach()` → 退出进程

### 2. 命令集规范

| 命令 | 语法 | 描述 | 有效状态 | 示例输出 |
|------|------|------|---------|---------|
| `halt` | `halt` | 暂停目标 | Running | `[OK] target halted` |
| `resume` | `resume` | 全速运行 | Halted | `[OK] target running` |
| `step` | `step` | 单步执行 | Halted | `[OK] stepped to 0x08000104` |
| `break` | `break <addr>` | 设硬件断点 | Halted | `[#0] breakpoint at 0x08000100` |
| `regs` | `regs` | 显示寄存器 | Halted | `r0 = 0x00000000, r1 = 0x00000001, ...` |
| `mem` | `mem <addr> <len>` | 读取内存 | Halted | 十六进制 + ASCII dump |
| `status` | `status` | 显示会话状态 | 任何 | `HALTED | chip=STM32F407VG | bp=1 | pc=0x08000104` |
| `help` | `help` | 显示帮助 | 任何 | 命令列表 + 用法 |
| `quit` | `quit` / `exit` | 退出会话 | 任何 | — |

### 3. 异常与阻断流

| 失败场景 | 用户可见消息 | 系统行为 |
|---------|------------|---------|
| ELF 文件不存在 | `"ELF file not found: {path}"` | 报错退出 (exit=1) |
| 芯片未知 | `"unknown chip '{name}'"` | 报错退出 |
| 探针无法连接 | `"probe-rs attach failed: {reason}"` | 报错退出 |
| break 地址无效 | `"set breakpoint at 0x{addr:08x} failed: {reason}"` | 打印错误，继续 REPL |
| mem 参数非数字 | `"invalid address: {input}"` | 打印错误，继续 REPL |
| 命令不在合法状态 | `"command 'step' not valid in Running state"` | 打印错误，继续 REPL |
| 探针意外断连 | `"probe disconnected, aborting"` | 退出 REPL |
| 空行 / 空白 | — | 静默忽略，重新提示 |

---

## 三、 架构设计方案 (Architecture Design)

### 3.1 `Session` 扩展

在已有 `Session` 结构体中新增 `backend` 字段，添加 `attach()` 静态构造函数。

```rust
// src/session.rs
pub struct Session {
    pub state: SessionState,
    pub chip_name: String,
    pub core_count: usize,
    pub pc: Option<u32>,
    pub bp_count: usize,
    pub watch_count: usize,
    pub backend: Box<dyn DebugProbe>,   // ← 新增
}

impl Session {
    /// 连接探针并创建会话（初始状态 Halted）
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

    /// 安全 detach（drop 前调用）
    pub fn detach(&mut self) -> anyhow::Result<()> {
        self.backend.detach()
    }
}
```

**注意**: `Session::new()` 现有构造函数保留但标记为弃用，`Session::attach()` 成为标准入口。

### 3.2 `Command` 枚举 + 解析

```rust
// src/cli/debug.rs
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

impl Command {
    /// 从用户输入的字符串解析命令。
    /// 返回 Err 时包含人类可读的错误消息。
    pub fn parse(input: &str) -> Result<Self, String> { ... }

    /// 该命令在哪些会话状态下合法（None = 所有状态）。
    pub fn valid_states(&self) -> Option<&[SessionState]> { ... }
}
```

解析规则：
- `trim()` + `to_lowercase()` 后以空格分割
- 命令名匹配：`halt` / `resume` / `step` / `break` / `regs` / `mem` / `status` / `help` / `quit` / `exit`
- 地址参数：支持 `0x` 前缀十六进制和纯十进制
- 长度参数：仅十进制
- 命令名不匹配 → `Err("unknown command '{input}'. Type 'help' for available commands.")`
- 参数个数不匹配 → `Err("usage: break <addr>")`

### 3.3 `DebugRepl` 结构体

```rust
pub struct DebugRepl {
    session: Session,
    rl: rustyline::Editor<()>,
}

impl DebugRepl {
    /// 创建 REPL 实例。
    pub fn new(session: Session) -> Self { ... }

    /// 进入主交互循环，直至用户 quit 或出现致命错误。
    pub fn run(&mut self) -> anyhow::Result<()> { ... }

    // ── 内部方法 ──

    /// 读取一行输入，尝试解析为 Command。
    fn read_command(&mut self) -> Option<Command> { ... }

    /// 检查命令在当前状态下是否合法，若合法则执行。
    fn execute(&mut self, cmd: Command) -> anyhow::Result<()> { ... }

    // ── 每条命令对应方法 ──

    fn cmd_halt(&mut self) -> anyhow::Result<()> { ... }
    fn cmd_resume(&mut self) -> anyhow::Result<()> { ... }
    fn cmd_step(&mut self) -> anyhow::Result<()> { ... }
    fn cmd_break(&mut self, addr: u32) -> anyhow::Result<()> { ... }
    fn cmd_regs(&mut self) -> anyhow::Result<()> { ... }
    fn cmd_mem(&mut self, addr: u32, len: u32) -> anyhow::Result<()> { ... }
    fn cmd_status(&self) { ... }
    fn print_help(&self) { ... }
}
```

### 3.4 主循环逻辑（`run()`）

```
loop {
    match self.read_command() {
        Some(Quit) => break,
        Some(cmd) => {
            // 状态守卫检查
            if let Some(states) = cmd.valid_states() {
                if !states.contains(&self.session.state) {
                    println!("command '{cmd}' not valid in {:?} state", self.session.state);
                    continue;
                }
            }
            // 执行
            if let Err(e) = self.execute(cmd) {
                println!("[ERROR] {e}");
                // 非致命错误继续循环
            }
        }
        None => continue,  // 空行或 readline 错误
    }
}
// 清理
self.session.detach()?;
println!("[OK] debug session ended");
```

### 3.5 格式化输出约定

| 输出类型 | 前缀 | 示例 |
|---------|------|------|
| 成功操作 | `[OK]` | `[OK] breakpoint #0 set at 0x08000100` |
| 断点命中 | 无前缀 | `** breakpoint #0 at 0x08000100 **` |
| 错误（可恢复） | `[ERROR]` | `[ERROR] invalid address: abc` |
| 寄存器 | tab 分隔 | `r0    0x00000000` |
| 内存 | 地址 + 十六进制 | `0x20000000  00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f` |
| 状态 | key=value | `state=Halted  chip=STM32F407VG  bp=1  pc=0x08000104` |

---

## 四、 测试策略 (Test Strategy)

### 命令解析单元测试

| 测试函数 | 输入 | 预期 |
|---------|------|------|
| `test_parse_halt` | `"halt"` | `Ok(Command::Halt)` |
| `test_parse_resume` | `"resume"` | `Ok(Command::Resume)` |
| `test_parse_step` | `"step"` | `Ok(Command::Step)` |
| `test_parse_break` | `"break 0x08000100"` | `Ok(Command::Break { addr: 0x08000100 })` |
| `test_parse_break_decimal` | `"break 134219776"` | `Ok(Command::Break { addr: 0x08000100 })` |
| `test_parse_break_no_addr` | `"break"` | `Err` |
| `test_parse_break_bad_addr` | `"break abc"` | `Err` |
| `test_parse_regs` | `"regs"` | `Ok(Command::Regs)` |
| `test_parse_mem` | `"mem 0x20000000 16"` | `Ok(Command::Mem { addr: 0x20000000, len: 16 })` |
| `test_parse_mem_missing_len` | `"mem 0x20000000"` | `Err` |
| `test_parse_status` | `"status"` | `Ok(Command::Status)` |
| `test_parse_help` | `"help"` | `Ok(Command::Help)` |
| `test_parse_quit_exit` | `"quit"`, `"exit"` | `Ok(Command::Quit)` |
| `test_parse_unknown` | `"xyz"` | `Err` |
| `test_parse_whitespace` | `"  "`, `""` | `Err` 或 `None` 语义 |

### 状态守卫测试

| 测试函数 | 命令 | 会话状态 | 预期 |
|---------|------|---------|------|
| `test_halt_valid_in_running` | `Halt` | Running | 可执行 |
| `test_halt_invalid_in_halted` | `Halt` | Halted | 拒绝 |
| `test_resume_valid_in_halted` | `Resume` | Halted | 可执行 |
| `test_resume_invalid_in_running` | `Resume` | Running | 拒绝 |
| `test_step_valid_in_halted` | `Step` | Halted | 可执行 |
| `test_step_invalid_in_running` | `Step` | Running | 拒绝 |
| `test_break_valid_in_halted` | `Break` | Halted | 可执行 |
| `test_status_all_states` | `Status` | 任何 | 可执行 |
| `test_help_all_states` | `Help` | 任何 | 可执行 |
| `test_quit_all_states` | `Quit` | 任何 | 可执行 |

### 集成测试（需要真实硬件，标记 `#[ignore]`）

| 测试函数 | 描述 |
|---------|------|
| `test_debug_elf_not_found` | 不存在的 ELF 路径 → Err |
| `test_debug_unknown_chip` | 无效芯片名 → Err |
| `test_debug_no_config` | 无 chip、无配置文件 → Err |

---

## 五、 验收断言与 Definition of Done

- [ ] **1. CLI 入口可达**: `cargo run -- debug --elf fw.elf --chip STM32F407VG` 启动后进入 `(mcu) > ` 提示符
- [ ] **2. 9 条命令可识别**: `halt` / `resume` / `step` / `break <addr>` / `regs` / `mem <addr> <len>` / `status` / `help` / `quit` 各自触发对应行为
- [ ] **3. 状态守卫工作**: Halted 态调 `halt` → 拒绝；Running 态调 `step` → 拒绝；Status 任何态均可
- [ ] **4. 错误隔离**: 无效参数 / 无效命令 → 打印错误不崩溃，继续 REPL 循环
- [ ] **5. 命令解析测试通过**: 15 个命令解析单元测试全部通过
- [ ] **6. 格式合规**: `cargo fmt --all -- --check` 零差异
- [ ] **7. 测试套件通过**: `cargo test -- --skip test_attach_without_hardware` 全部通过
- [ ] **8. 回归防御**: 已有 17 个测试不变，全部通过
- [ ] **9. context.md 同步**: 变更日志已追加

---

## 六、 受影响文件清单 (Affect Map)

> ⚠ 此清单仅列出受影响的文件，**不是执行顺序**。执行顺序由后续 `/code-spec` 生成的 `task.md` 定义。

| 文件 | 改动类型 | 说明 |
|------|---------|------|
| `src/session.rs` | 🟡 扩展 | 新增 `backend: Box<dyn DebugProbe>` 字段；新增 `Session::attach()` 静态构造函数；保留 `new()` 标记弃用 |
| `src/cli/debug.rs` | 🟢 实现 | 新增 `Command` enum + `parse()` + `valid_states()`；新增 `DebugRepl` 结构体 + 所有方法；实现 `handle()` |
| `src/main.rs` | ⬜ 可能微调 | 检查 `Commands::Debug` 解构→传入 `DebugArgs` 的流程（目前已有，可能不需要改动） |
| `src/probe/probe_rs.rs` | ⬜ 不动 | 所有方法已实现 |
| `Cargo.toml` | ⬜ 不动 | `rustyline` 已存在 |
