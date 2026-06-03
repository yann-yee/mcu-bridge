# task.md - 精细化函数级开发任务清单与执行足迹

> ⓘ 本文件是实现特性的 AI 代理的核心执行手记。你必须保持在任意时刻**只能有 1 个任务处于 [in-progress] 状态**。

---

## 📌 当前总览
- **源需求文档**: [user_plan/ci-cd/ci-cd.md](user_plan/ci-cd/ci-cd.md)
- **最新更新日期**: 2026-06-03 (已归档)
- **整体进度状态**: `completed`

---

## 一、 重写 CI 主流水线 (ci.yml)

- [x] **Task 1.1: 重写 `.github/workflows/ci.yml`**
  - **受影响文件**: `[.github/workflows/ci.yml](.github/workflows/ci.yml)`
  - **实施计划**:
    1. 保留 `name: CI`、`on: push/pull_request`、`env`
    2. **fmt job**: 保持现有（Linux only），加 `timeout-minutes: 5`
    3. **clippy job**: 保持现有，加 `uses: Swatinem/rust-cache@v2` 步骤（在 setup-rust-toolchain 之后），加 `timeout-minutes: 10`
    4. **build-and-test job**: 新建 matrix job 替换原 `unit-test` + `integration`：
       - `strategy.matrix.os: [ubuntu-latest, windows-latest, macos-latest]`
       - `runs-on: ${{ matrix.os }}`
       - steps: checkout → setup-rust-toolchain (1.95) → rust-cache →
         - `if: runner.os == 'Linux'`: `sudo apt-get install -y libudev-dev libusb-1.0-0-dev`
         - `run: cargo generate-lockfile` (first)
         - `run: cargo build --release`
         - `run: cargo test --lib`
    5. 删除原 `integration` 占位 job
  - **本地验证命令**: 语法审查（`grep -c "runs-on\|strategy\|matrix\|rust-cache\|apt-get" .github/workflows/ci.yml`）
  - **当前状态**: `completed`

---

## 二、 新建 Release 流水线 (release.yml)

- [x] **Task 2.1: 新建 `.github/workflows/release.yml`**
  - **受影响文件**: `[.github/workflows/release.yml](.github/workflows/release.yml)`（新文件）
  - **实施计划**:
    1. `name: Release`、`on: push.tags: ["v*"]`
    2. **build job** (matrix):
       - `strategy.matrix.include` 三个平台，每个包含 `os`、`target`（Rust target triple）、`ext`（tar.gz/zip）
       - steps: checkout → setup-rust-toolchain(1.95) → rust-cache → `cargo build --release`
       - `- uses: actions/upload-artifact@v4` 上传二进制为 artifact（跨 job 传递文件）
    3. **release job**:
       - `needs: build`、`runs-on: ubuntu-latest`
       - steps: 下载所有 artifact → 打包重命名为 `mcu-bridge-{target}.{ext}` →
       - `- uses: softprops/action-gh-release@v2` 上传所有文件，附带 `body: ${{ github.ref_name }}` 作为 release note
  - **本地验证命令**: `test -f .github/workflows/release.yml && echo "exists"` + 语法审查
  - **当前状态**: `completed`

---

## 三、 更新项目宪法 (AGENTS.md §三)

- [x] **Task 3.1: 更新 `AGENTS.md §三` CI/CD 约束章节**
  - **受影响文件**: `[AGENTS.md](AGENTS.md)` (行 148-167)
  - **实施计划**:
    1. 更新 §三 标题为 "三、CI/CD 约束 (GitHub Actions)"（不变）
    2. 在 "CI 流水线步骤" 列表中：
       - 保留 1-4 步（fmt/clippy/unit-test/integration），将 "集成测试" 描述从 "Docker 环境" 改为 "Docker 环境（当前占位待 P2 启用）"
       - 新增第 5 步：`**跨平台构建**: cargo build --release on ubuntu-latest + windows-latest + macos-latest（matrix job）`
       - 新增第 6 步：`**缓存**: Swatinem/rust-cache@v2（基于 Cargo.lock hash）`
    3. 在 "CI 通过的不可妥协条件" 后追加 release 说明：`- Tag push (v*) 自动触发 release 流水线（.github/workflows/release.yml），产出三平台二进制并发布到 GitHub Release。`
  - **本地验证命令**: `grep -c "release\|rust-cache\|matrix\|三平台" AGENTS.md`
  - **当前状态**: `completed`

---

## 四、 全量验证

- [x] **Task 4.1: 生成 Cargo.lock + 本地全量验证**
  - **描述**: 确保 `Cargo.lock` 存在且包含 CI 所需的 hash，本地回归验证代码无退化。
  - **执行命令**:
    1. `cargo generate-lockfile`  → 生成 `Cargo.lock`
    2. `cargo check` → 零 warning
    3. `cargo fmt --all -- --check` → 通过
    4. `cargo test --lib` → 10 passed
  - **当前状态**: `completed`

- [x] **Task 4.2: git commit + push + Actions 验证**
  - **描述**: 提交所有变更（ci.yml、release.yml、Cargo.lock、AGENTS.md）push 到 GitHub，在 Actions tab 中确认 CI 三平台 job 全绿。
  - **执行命令**: `git add . && git commit -m "..." && git push`
  - **当前状态**: `completed`
