# spec.md - 编码过程沙盒保险箱与红线契约

> ⓘ 本文件是本次代码重构的「保险圈与硬约束控制中心」。后续 Agent 在进行编码时必须 100% 保持在这些红线圈内。

---

## 一、 控制沙盒与严禁篡改红线 (Strict Boundary / Do Not Touch)

绝对禁止 Agent 修改或影响的区域与架构规则：

- **红线 1 [Rust 源代码 100% 冻结]**: 
  - 本次变更仅涉及 `.github/workflows/` 下的 YAML 文件和 `AGENTS.md §三`。严禁修改 `src/` 下的任何 `.rs` 文件、`Cargo.toml`、`rustfmt.toml`、`.gitignore`。
  - 唯一例外：如果 CI 编译失败需要调整代码（如添加 `#[cfg(test)]` 注解），必须先报告并请求用户批准。

- **红线 2 [GitHub Actions 依赖白名单]**: 
  - 只允许使用需求文档 §4 列出的 4 个第三方 action：
    - `actions/checkout@v4`
    - `actions-rust-lang/setup-rust-toolchain@v1`
    - `Swatinem/rust-cache@v2`
    - `softprops/action-gh-release@v2`
  - 严禁引入其他 action（如 `actions-rs/cargo`、`docker/build-push-action` 等）。

- **红线 3 [Cargo.lock 提交不可跳过]**: 
  - 二进制项目必须提交 `Cargo.lock`。CI 中需在 `cargo build` 前显式 `cargo generate-lockfile`（若 lock 文件缺失）确保缓存 key 一致。

- **红线 4 [AGENTS.md §三 增量更新]**: 
  - 更新 CI/CD 约束章节时，保留原有的四步不可妥协条件（fmt/clippy/test/integration），在此基础上追加三平台矩阵和 release 说明。禁止删减或改写原有约束。

---

## 二、 编码设计规范

- **YAML 缩进**: 2 空格，无 tab。
- **Job 命名**: kebab-case（`build-and-test`、`unit-test`）。
- **Step 命名**: 每个 `- name:` 写中文描述 + 英文命令注释。

---

## 三、 本次开发的硬防崩溃约束

- 1. **Linux 系统依赖**: `build-and-test` job 的 Linux runner 必须在 checkout 之后、cargo build 之前运行 `sudo apt-get install -y libudev-dev libusb-1.0-0-dev`。
- 2. **matrix 变量传递**: `strategy.matrix.os` 的 `os` 值用于条件执行（如 `if: runner.os == 'Linux'`），不要硬编码 runner 名称。
- 3. **release 上传重名**: 三平台 build job 产出不同 target triple 的二进制，确保文件名含 `${{ matrix.target }}` 以避免同名覆盖。

---

## 四、 本次规范验收评估核对

- [ ] `cargo check` 零 warning（代码未变，回归验证）
- [ ] `cargo fmt --all -- --check` 通过
- [ ] `cargo test --lib` 全部绿色
- [ ] 两个 YAML 文件语法正确（`on`、`jobs`、`steps` 无错别字）
