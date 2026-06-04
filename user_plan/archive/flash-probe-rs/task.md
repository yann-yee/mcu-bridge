# task.md - 精细化函数级开发任务清单与执行足迹

> ⓘ 本文件是实现特性的 AI 代理的核心执行手记。每一个步骤都精确写明了受影响文件、拟添加/修改的方法名称。你必须保持在任意时刻**只能有 1 个任务处于 [in-progress] 状态**；完成一步后，运行对应的验证命令并打勾，再推进下一步，保证开发路径 100% 可回溯。

---

## 📌 当前总览

- **源需求文档**: [user_plan/flash-probe-rs/flash-probe-rs.md](flash-probe-rs.md)
- **最新更新日期**: 2026-06-04
- **整体进度状态**: `completed`

---

## 一、 开发准备与依赖准备 (Preparation)

- [x] **Task 1.1: 运行项目基础构建与测试校验**
  - **描述**: 证明在此次开发前，项目本地环境绝对完好。
  - **本地执行检验命令**: `cargo check && cargo test --lib`
  - **验证结果**: `cargo check` ✅, `cargo test` 10/10 ✅

---

## 二、 核心实体与 CLI 参数层改动 (Data Layer)

- [x] **Task 2.1: 为 `Commands::Flash` 变体新增 `--run` 参数**
  - **受影响文件**: `[src/cli/mod.rs](src/cli/mod.rs)`
  - **函数/属性级实施计划**:
    1. 在 `Commands::Flash` 的字段列表中追加 `#[arg(long)] run: bool`。
  - **验证结果**: `cargo check` ✅, `cargo run -- flash --help` 显示 `--run` ✅

- [x] **Task 2.2: 在 `main.rs` 的 `Commands::Flash` destructure 中同步追加 `run`**
  - **受影响文件**: `[src/main.rs](src/main.rs)`
  - **验证结果**: `cargo check` ✅

- [x] **Task 2.3: 提升 `get_chip_template` 可见性为 `pub(crate)`**
  - **受影响文件**: `[src/cli/init.rs](src/cli/init.rs)`
  - **验证结果**: `cargo check` ✅

- [x] **Task 2.4: 为 `FlashArgs` 新增 `run` 字段**
  - **受影响文件**: `[src/cli/flash.rs](src/cli/flash.rs)`
  - **验证结果**: `cargo check` ✅

---

## 三、 核心后端业务逻辑实现 (Backend Services & Controllers)

- [x] **Task 3.1: 实现 `resolve_chip_config()` 芯片配置解析函数**
  - **受影响文件**: `[src/cli/flash.rs](src/cli/flash.rs)`
  - **函数级实施计划**:
    1. `fn resolve_chip_config(chip_arg: Option<&str>) -> anyhow::Result<(ChipConfig, FlashOpts)>`
    2. `--chip` 优先 → `init::get_chip_template()`；回退 `.debugger/chip.toml` → `toml::from_str`
  - **验证结果**: `cargo test test_flash_unknown_chip`, `test_flash_no_chip_no_config` ✅

- [x] **Task 3.2: 重写 `handle()` 为真实 Standalone 烧录流程**
  - **受影响文件**: `[src/cli/flash.rs](src/cli/flash.rs)`
  - **函数级实施计划**:
    1. ELF 校验 → 芯片配置 → attach → flash → 可选 resume → detach
    2. 进度走 `eprintln!`，结果走 `println!`
  - **验证结果**: `cargo check && cargo test --lib` 13/13 ✅

---

## 四、 测试层 (Test Coverage)

- [x] **Task 4.1: 新增 flash 子命令错误路径单元测试**
  - **受影响文件**: `[src/cli/flash.rs](src/cli/flash.rs)`
  - **函数级实施计划**:
    1. `test_flash_elf_not_found` — ELF 不存在 → Err
    2. `test_flash_unknown_chip` — 无效芯片名 → Err
    3. `test_flash_no_chip_no_config` — 无配置 → Err
  - **验证结果**: `cargo test test_flash_` 3/3 ✅

---

## 五、 全局集成检验与 DoD 验证机制 (Whole Loop Verification)

- [x] **Task 5.1: 全量验证套件**
  - **验证结果**:
    1. `cargo check` → 零 error ✅
    2. `cargo fmt --all -- --check` → 零差异 ✅
    3. `cargo test` → 13/13 全部绿色 ✅
    4. `cargo run -- flash --help` → 包含 `--run`、`--verify`、`--chip` ✅
    5. ELF 不存在 → `"ELF file not found"`, exit=1 ✅
    6. 未知芯片 → `"unknown chip"`, exit=1 ✅
    7. 无配置 → `"cannot read .debugger/chip.toml"`, exit=1 ✅
