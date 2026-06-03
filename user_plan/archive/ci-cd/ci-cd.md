# 需求规格说明书：CI/CD 流水线升级 — 三平台矩阵 + 自动发布 + 缓存优化

> ⓘ 本文档已经过 "Understanding (Grill Me)" 技能对齐，记录了逻辑共识。本文件已于 [user_plan/ci-cd/ci-cd.md](user_plan/ci-cd/ci-cd.md) 归档。实现该功能的后续 Agent 必须全盘以此文档为验收终点。

---

## 一、 功能概览与基本愿景 (User Story & Goal)

- **新功能背景**: 当前 CI 骨架（`.github/workflows/ci.yml`）只有 4 个 job 在 `ubuntu-latest` 单平台上运行——fmt、clippy、unit-test、以及一个占位 integration job。无缓存、无跨平台验证、无发布流水线。经过两轮 P0 开发后，代码库已从骨架变为有真实业务逻辑的项目（probe-rs backend + CLI 子命令），需要 CI 升级到具备跨平台保障和自动发布能力的成熟度。
- **用户故事 (User Story)**: 作为一名开源用户/Agent，我想要在 `mcu-bridge` 仓库的 GitHub Release 页面上直接下载我所在平台（Windows/Linux/macOS）的预编译二进制文件，且每次 PR 合并前 CI 能自动在三平台上验证代码质量，以便我无需安装 Rust 工具链即可使用该工具。
- **关联已有的技术链**:
  - `.github/workflows/ci.yml` — 当前骨架 CI（4 job，ubuntu only，无缓存），本次全量替换
  - `Cargo.toml` — `probe-rs = "0.31"` 依赖树 ~250 crate，缓存策略取决于此
  - `AGENTS.md §三` — CI/CD 约束章节，定义了 fmt/clippy/test/integration 四步和不可妥协条件
  - `context.md` — 项目定位（嵌入式调试中间件、CLI + JSON-Lines）

---

## 二、 极限对齐的业务流程 (Consensus Flow & Sequence)

> 记录烤问阶段彻底敲定的顺畅流与异常阻断流。

### 1. 标准顺畅流 (Happy Path)

**PR 提交流（push/PR to main）**：
1. 开发者 push 到分支或创建 PR → CI 触发
2. `fmt` job — Linux 单平台，`cargo fmt --all -- --check`，耗时 < 10s
3. `clippy` job — Linux 单平台，`cargo clippy --all-targets --all-features -- -D warnings`，耗时 ~30-60s
4. `build-and-test` job — 三平台矩阵（Linux/Windows/macOS），每平台执行：`cargo build --release` + `cargo test --lib`
5. 四 job 全部通过 → PR 可合并

**Tag 发布流（push tag v*）**：
1. 开发者打 tag 并 push → release CI 触发
2. 三平台并行 `cargo build --release` → 产出三个二进制
3. 创建 GitHub Release（用 `softprops/action-gh-release`，自动附上 CHANGELOG 或 tag message）
4. 二进制文件打包为 `mcu-bridge-{target}.tar.gz`（Linux/macOS）或 `mcu-bridge-{target}.zip`（Windows）
5. Release 页面自动出现三个平台的可下载二进制

**缓存流**：
1. 每个 job 开始时 `Swatinem/rust-cache@v2` 自动检查 `Cargo.lock` hash
2. 命中 → 直接复用 `~/.cargo/registry` 和增量编译产物（保存 ~1-2 分钟）
3. 未命中（首次或 Cargo.lock 变更）→ 全量下载 + 编译 + 自动保存缓存

### 2. 异常与阻断流 (Exception Handlings)

- **probe-rs feature 差异导致编译失败**: 如果 CI 环境中缺少 Linux 的系统库（如 `libudev-dev`、`libusb-1.0-0-dev`），probe-rs 的 `hidapi`/`libusb` 依赖编译失败。CI 中需预装 `libudev-dev libusb-1.0-0-dev`（Linux）。
- **macOS 交叉编译不适用**: probe-rs 含有 C 依赖（hidapi、nusb），不能在 Linux 上用 `cross` 交叉编译到 macOS。三平台必须各用各的原生 runner——`ubuntu-latest`、`windows-latest`、`macos-latest`。
- **release job 与 PR job 缓存隔离**: tag push 和 PR push 可能同时触发。两者共享缓存 key（基于 Cargo.lock hash），但 PR job 不应污染 release binary 的增量编译。rust-cache 的 key 已包含 job name，天然隔离。
- **Cargo.lock 不存在**: 项目当前未提交 `Cargo.lock`（`.gitignore` 中未过滤但文件尚未生成）。CI 中应确认 lock 文件存在性——若缺失，`cargo generate-lockfile` 自动生成。

---

## 三、 烤问决策记录 (Grill Decisions)

本需求在 Understanding 阶段经历了三轮极限追问。以下为所有敲定的技术分歧点：

### 🔧 决策 1：跨平台矩阵 → 三平台全覆盖

- **实现**: `build-and-test` job 用 `strategy.matrix.os = [ubuntu-latest, windows-latest, macos-latest]`。fmt 和 clippy 仅在 Linux 跑（结果跨平台一致）。
- **理由**: AGENTS.md §3.3 已要求条件编译兼容双平台，没有 CI 自动化验证意味着条件编译分支永不被测试。macOS 虽当前无用户场景，但 probe-rs 官方支持 macOS FTDI/CMSIS-DAP，三平台一起覆盖可以一次 CI 配置到位，省得未来补加。
- **否定方案**: 仅 Linux（Windows bug 漏检）、Linux+Windows（macOS 未来再补成本更高）

### 🔧 决策 2：发布策略 → CI 自动构建 + GitHub Release（tag 触发）

- **实现**: 新增 `release.yml`（仅在 `on.push.tags: ["v*"]` 触发），三平台 `cargo build --release` → 用 `softprops/action-gh-release@v2` 自动上传二进制到 GitHub Release。
- **理由**: 目标用户（AI Agent、嵌入式开发者）需要下载即用的二进制而非 `cargo install`（编译 3-5 分钟）。二进制发布是 CLI 工具的开源标准做法。
- **否定方案**: 手动发布（每次多平台编译繁琐）、crates.io（编译等待体验差，现阶段 API 不稳定不宜 publish）

### 🔧 决策 3：缓存策略 → Swatinem/rust-cache@v2

- **实现**: 每个 job 的 setup-rust-toolchain 之后加 `uses: Swatinem/rust-cache@v2`。缓存 key 基于 Cargo.lock hash + job name。
- **理由**: Rust 社区 CI 缓存事实标准，一行引用零配置。缓存 registry 而非 target/ 是正确策略——target/ 跨平台不通用且体积大，registry 缓存已消除大部分下载时间。
- **否定方案**: 裸跑（浪费分钟数）、手写 actions/cache（维护成本高）

---

## 四、 技术契约定义 (Technical Contract)

### 4.1 CI 文件结构

两个工作流文件：

| 文件 | 触发条件 | 职责 |
|------|---------|------|
| `.github/workflows/ci.yml` | push/PR to `main` | fmt + clippy + 三平台 build+test |
| `.github/workflows/release.yml` | tag push `v*` | 三平台 `cargo build --release` + GitHub Release |

### 4.2 ci.yml job 清单

| Job | Runner | 缓存 | 命令 |
|-----|--------|------|------|
| `fmt` | `ubuntu-latest` | ❌ 不需要 | `cargo fmt --all -- --check` |
| `clippy` | `ubuntu-latest` | ✅ rust-cache | `cargo clippy --all-targets --all-features -- -D warnings` |
| `build-and-test` | `[ubuntu-latest, windows-latest, macos-latest]` (strategy.matrix) | ✅ rust-cache | `cargo build --release` + `cargo test --lib` |

### 4.3 release.yml job 清单

| Job | Runner | 产出 |
|-----|--------|------|
| `build` | `[ubuntu-latest, windows-latest, macos-latest]` (strategy.matrix) | 编译产物 |
| `release` | `ubuntu-latest` (needs: build) | GitHub Release + 上传所有平台二进制 |

二进制命名：`mcu-bridge-{target}.{ext}`，其中 target 为 Rust target triple（`x86_64-unknown-linux-gnu` / `x86_64-pc-windows-msvc` / `x86_64-apple-darwin`），ext 为 `tar.gz`（Linux/macOS）或 `zip`（Windows）。

### 4.4 Linux 系统依赖

probe-rs 的 hidapi 和 libusb 需要系统库。CI 中 `ubuntu-latest` runner 需预装：

```bash
sudo apt-get update && sudo apt-get install -y libudev-dev libusb-1.0-0-dev
```

---

## 五、 验收断言与 Harness 测试指标 (Definition of Done)

> 绝对禁止空洞通过。以下每条都必须通过命令或 CI 日志验证。

- [ ] **1. ci.yml 文件存在且结构正确**: `.github/workflows/ci.yml` 包含 `fmt`、`clippy`、`build-and-test`（matrix）三个 job。
- [ ] **2. release.yml 文件存在且结构正确**: `.github/workflows/release.yml` 含 tag 触发 + 三平台 build + GitHub Release 上传。
- [ ] **3. 缓存配置断言**: 所有需要编译的 job（clippy + build-and-test）都有 `uses: Swatinem/rust-cache@v2` 步骤。
- [ ] **4. Linux 系统依赖断言**: `build-and-test` job 的 Linux runner 有 `apt-get install libudev-dev libusb-1.0-0-dev` 步骤。
- [ ] **5. 本地 CI 语法验证**: 使用 `act` 或手动检查 YAML 语法无误（`yamllint` 或人工审查）。
- [ ] **6. CI 实际运行断言**: push 到 GitHub 后，Actions tab 中出现一次完整运行的 CI pipeline，`fmt`/`clippy`/`build-and-test` 三个 job 全部绿色（初版 integration 可跳过）。
- [ ] **7. matrix 三平台断言**: `build-and-test` job matrix 包含 `ubuntu-latest`、`windows-latest`、`macos-latest` 三个 runner，且各自有 `cargo build --release` + `cargo test --lib` 步骤。
- [ ] **8. release 二进制命名断言**: release job 产出的二进制名称为 `mcu-bridge-x86_64-unknown-linux-gnu.tar.gz` 等格式。
