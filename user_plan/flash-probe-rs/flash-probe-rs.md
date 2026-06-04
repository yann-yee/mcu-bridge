# 需求规格说明书：probe-rs 烧录功能实现

> ⓘ 本文档已经过 "Understanding (Grill Me)" 技能对齐，记录了逻辑共识。本文件已于 [user_plan/flash-probe-rs/flash-probe-rs.md](user_plan/flash-probe-rs/flash-probe-rs.md) 归档。实现该功能的后续 Agent 必须全盘以此文档为验收终点。

---

## 一、 功能概览与基本愿景 (User Story & Goal)

- **新功能背景**: `mcu-bridge flash` 子命令当前是 P0 dry-run 占位——仅校验 ELF 文件存在后打印诊断信息就退出。经过 P0 阶段，probe-rs backend 的 `DebugProbe::flash()`、`attach()`、`detach()` 方法已有真实实现。现在需要将 CLI 层与 backend 层串联起来，使 `mcu-bridge flash --elf fw.elf` 能完成真正的烧录操作。
- **用户故事 (User Story)**: 作为一名嵌入式开发者或 AI Agent，我想要执行 `mcu-bridge flash --elf target/firmware.elf --chip STM32F407VG` 后，工具能自动连接调试探针、烧录固件到目标芯片、并进行回读校验，以便我无需手写 OpenOCD 命令或 J-Flash 即可完成固件部署。
- **关联已有的技术链**:
  - `src/probe/probe_rs.rs` — `ProbeRsBackend::flash()` 已调用 `probe_rs::flashing::download_file`（真实实现）
  - `src/probe/mod.rs` — `DebugProbe` trait 的 `attach/detach/flash` 签名已就位
  - `src/cli/flash.rs` — 当前为 dry-run（P0 占位）
  - `src/cli/mod.rs` — `Flash` 变体含 `--elf`、`--verify`、`--chip` 参数，本次需新增 `--run`
  - `src/cli/init.rs` — `get_chip_template()` 函数（需提升为 `pub(crate)` 供 flash 调用）
  - `src/config.rs` — `ChipConfig`、`FlashOpts`、`AppConfig` 类型体系已就位

---

## 二、 极限对齐的业务流程 (Consensus Flow & Sequence)

> 记录烤问阶段彻底敲定的顺畅流与异常阻断流。

### 1. 标准顺畅流 (Happy Path)

**`mcu-bridge flash --elf fw.elf --chip STM32F407VG`**：
1. CLI 解析 `--elf fw.elf`（文件存在校验）、`--chip STM32F407VG`、`--verify`（默认 true）
2. 通过 `cli::init::get_chip_template("STM32F407VG")` 获取芯片配置（Flash 基址 0x08000000、大小 1MB、RAM 128KB）
3. 创建临时 `ProbeRsBackend` → `backend.attach(&chip)` 连接探针
4. stderr 输出 `[INFO] attaching probe to STM32F407VG...`
5. `backend.flash(&elf, &flash_opts)` 执行烧录（probe-rs `download_file`）
6. stdout 输出 `[OK] flash complete`
7. `backend.detach()` 断开连接
8. 默认 halt 状态；若传了 `--run`，烧录后执行 `backend.resume(None)` 使目标自动运行

**芯片配置来源（按优先级递减）**：
1. `--chip` 命令行参数 → 从模板库 `get_chip_template()` 获取
2. `.debugger/chip.toml` 配置文件 → `toml::from_str` 反序列化 `AppConfig`
3. 两者都未提供 → 报错退出：`"no chip specified: use --chip or create .debugger/chip.toml"`

**`mcu-bridge flash --elf fw.elf`（从 `.debugger/chip.toml` 读取）**：
1. 解析 `--elf fw.elf`
2. 自动查找当前目录下的 `.debugger/chip.toml`
3. 读取 `[chip]` 和 `[flash]` section
4. 后续 attach → flash → detach 同上

### 2. 异常与阻断流 (Exception Handlings)

- **ELF 文件不存在**: `Path::exists()` 校验失败 → `anyhow::bail!("ELF file not found: ...")`
- **芯片模板不存在**: `get_chip_template()` 返回 `unknown chip 'XXX'` → 报错退出
- **`.debugger/chip.toml` 不存在**: 无 `--chip` 时的回退路径，文件不存在 → 报错 `"cannot read .debugger/chip.toml: ..."`
- **探针连接失败**: `attach()` 调用 probe-rs API 失败 → 错误链包含 probe-rs 原始错误信息
- **烧录失败**: `download_file` 失败 → 错误链包含 probe-rs 原始错误信息（如 Flash 擦除失败、数据校验失败）
- **探针未连接**: 所有 backend 方法都有 `ok_or_else(|| anyhow::anyhow!("not attached"))` 的空值防护
- **detach 幂等**: `detach()` 将 `session` 设为 `None`，多次调用安全

---

## 三、 烤问决策记录 (Grill Decisions)

本需求在 Understanding 阶段经历了两轮极限追问。以下为所有敲定的技术分歧点：

### 🔧 决策 1：烧录模式 → Standalone（即用即走）

- **实现**: `cli::flash::handle()` 内部创建临时 `ProbeRsBackend` → attach → flash → detach，完全自包含。
- **理由**: CLI 结构中 `flash` 是顶层子命令（与 `init`/`clean`/`debug` 平级），独立操作简单直观。若未来需要在 `debug` REPL 内烧录，可在该子命令中追加 `reflash` 指令。
- **否定方案**: Session 持久化复用（需要跨命令共享状态，过度复杂）

### 🔧 决策 2：芯片配置优先级 → `--chip` > `.debugger/chip.toml`

- **实现**: `resolve_chip_config()` 函数先检查 `--chip` 参数，有则从模板获取；无则读取 `.debugger/chip.toml`。
- **理由**: CLI 参数是用户最明确的意图表达；配置文件是 `init` 子命令的标准产出物，两者按此优先级兼容。
- **否定方案**: 仅配置文件（不灵活）、仅 `--chip`（忽略已有的 `chip.toml`）

### 🔧 决策 3：烧录后目标状态 → 默认 halt，`--run` 控制自动运行

- **实现**: 在 `Commands::Flash` 变体中新增 `#[arg(long)] run: bool` 参数。`handle()` 末尾判断：若 `run` 为 true，调用 `backend.resume(None)`；否则直接 detach（保持 halt）。
- **理由**: 开发调试场景需要 halt 后设断点检查；CI/部署场景需要烧完即跑。参数控制覆盖两种需求。
- **否定方案**: 统一 halt（部署场景不适用）、统一 run（开发场景需额外 halt）

### 🔧 决策 4：烧录校验 → `--verify` 默认 true，`--no-verify` 关闭

- **实现**: 保持现有 `#[arg(long, default_value_t = true)] verify: bool`。`FlashOpts.verify` 透传。AGENTS.md §3.1 强制要求回读校验，默认开启符合安全规范。
- **理由**: 无校验的 "fast flash" 路径会掩盖烧录失败，默认为安全选项。需要提速时可显式 `--no-verify`。
- **否定方案**: 默认关闭（违反 AGENTS.md 安全要求）、去掉 `--verify`（永远校验——缺乏灵活性）

### 🔧 决策 5：进度反馈 → stderr `[INFO]` 进度报告

- **实现**: 使用 `eprintln!` 输出 `[INFO] attaching probe to {chip}...`、`[INFO] flashing ELF...` 等中间进度。
- **理由**: stdout 保留给 JSON-Lines 协议输出（未来 `--json` 模式），stderr 走诊断信息是 Rust CLI 惯例（类似 `cargo`）。probe-rs 的 `download_file` 无进度回调 API，现阶段信息型日志已足够。
- **否定方案**: 静默输出（Human 用户看不到进度）、println 到 stdout（污染 JSON-Lines 输出）

### 🔧 决策 6：后端选择 → 仅 probe-rs（OpenOCD 等 P2）

- **实现**: flash 子命令硬编码使用 `ProbeRsBackend`，不添加 `--backend` CLI 参数。
- **理由**: OpenOCD backend 本身是 P2 目标（`openocd.rs` 所有方法为 `todo!()`），此时加 `--backend` 无真实实现。等待 P2 统一处理。
- **否定方案**: 加 `--backend openocd` 参数（参数已定义但实现缺失，给用户虚假期望）

---

## 四、 技术契约定义 (Technical Contract)

### 4.1 修改文件清单

| 文件 | 变更类型 | 变更内容 |
|------|---------|---------|
| `src/cli/mod.rs` | 修改 | `Commands::Flash` 变体中新增 `#[arg(long)] run: bool` |
| `src/cli/flash.rs` | 重写 | dry-run 占位替换为真实 attach → flash → detach 流程 |
| `src/cli/init.rs` | 修改 | `get_chip_template` 改为 `pub(crate) fn` |
| `src/cli/debug.rs` | 不修改 | — |

### 4.2 新增 CLI 参数

```rust
// src/cli/mod.rs — Commands::Flash 追加
Run {
    /// 烧录完成后自动复位运行（默认 halt）
    #[arg(long)]
    run: bool,
},
```

### 4.3 芯片配置解析逻辑

```
resolve_chip_config(chip_arg: Option<&str>) -> (ChipConfig, FlashOpts)
├── chip_arg = Some("STM32F407VG")
│   ├── init::get_chip_template("STM32F407VG") → ChipConfig
│   └── 从 ChipConfig 构建 FlashOpts（base/size/sections/verify=true）
│   └── return (chip, opts)
├── chip_arg = None
│   ├── std::fs::read_to_string(".debugger/chip.toml")? → 字符串
│   ├── toml::from_str::<AppConfig>(&content)? → AppConfig
│   └── return (app.chip, app.flash)
└── 两者都失败 → bail!("no chip specified: use --chip or create .debugger/chip.toml")
```

### 4.4 `handle()` 完整流程

```rust
fn handle(args: &FlashArgs) -> anyhow::Result<()> {
    1. 校验 args.elf.exists()，否则 bail
    2. resolve_chip_config(args.chip.as_deref())? → (chip, flash_opts)
    3. eprintln!("[INFO] attaching probe to {}...", chip.name)
    4. let mut backend = ProbeRsBackend::new()
    5. backend.attach(&chip)?
    6. eprintln!("[INFO] flashing ELF...")
    7. backend.flash(&args.elf, &flash_opts)?
    8. if args.run { backend.resume(None)? }
    9. backend.detach()?
    10. println!("[OK] flash complete")
    11. Ok(())
}
```

---

## 五、 验收断言与 Harness 测试指标 (Definition of Done)

> 绝对禁止空洞通过。以下每条都必须通过命令验证。

- [x] **1. 编译断言**: `cargo check` 零 error 零 warning（不含 `dead_code`——P2 模块的 `#![allow(dead_code)]` 仍存在）。
- [x] **2. fmt 断言**: `cargo fmt --all -- --check` 零差异。
- [x] **3. 无硬件 dry-run 测试（ELF 不存在）**: `cargo run -- flash --elf nonexistent.elf --chip STM32F407VG` → stderr 含 `"ELF file not found"`，exit code ≠ 0。
- [x] **4. 无硬件 dry-run 测试（芯片模板不存在）**: `cargo run -- flash --elf Cargo.toml --chip INVALID_CHIP` → stderr 含 `"unknown chip"`，exit code ≠ 0。
- [x] **5. 无硬件 dry-run 测试（无配置无参数）**: `cargo run -- flash --elf Cargo.toml`（无 `--chip`、无 `.debugger/chip.toml`）→ 报错提示需要 chip。
- [x] **6. 单元测试继续通过**: `cargo test --lib` 10 个测试全部绿色。
- [x] **7. CLI 参数断言**: `cargo run -- flash --help` 输出中包含 `--run`、`--verify`、`--chip`、`--elf`、`--no-verify`。
- [x] **8. 真实硬件集成测试（手动/CI）**: 连接 STM32F411RE + ST-Link，`cargo run -- flash --elf test_firmware/firmware.elf --chip STM32F411RE --run` → `[OK] flash complete` 烧录成功，芯片自动运行新固件。**2026-06-04 实测通过**。
