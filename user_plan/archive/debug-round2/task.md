# task.md — Debug Round 2 精细化函数级开发任务清单

> ⓘ 本文件是实现"CLI 启动集成 + Agent JSON-Lines 模式"的 AI 代理的核心执行手记。每一个步骤都精确写明了受影响文件、拟添加/修改的方法名称。你必须保持在任意时刻**只能有 1 个任务处于 [in-progress] 状态**；完成一步后，运行对应的验证命令并打勾，再推进下一步。

---

## 📌 当前总览

- **源需求文档**: [user_plan/debug-round2/debug-round2.md](debug-round2.md)
- **最新更新日期**: 2026-06-06
- **整体进度状态**: `completed`

---

## 一、 开发准备与依赖准备 (Preparation)

- [x] **Task 1.1: 确认项目基线状态**
  - **描述**: 确保当前项目在改动前所有测试通过、编译通过。
  - **本地验证命令**: `cargo test -- --skip test_attach_without_hardware && cargo fmt --all -- --check && cargo check`
  - **当前状态**: `completed`

---

## 二、 基础设施层改动 (Session + Module Registration)

- [x] **Task 2.1: 修改 `Session::attach()` 签名 — 接受外部 backend 注入**
  - **受影响文件**: `[src/session.rs](../../src/session.rs)`
  - **函数级实施计划**:
    1. 将 `pub fn attach(chip: &ChipConfig) -> anyhow::Result<Self>` 改为 `pub fn attach(chip: &ChipConfig, backend: Box<dyn DebugProbe>) -> anyhow::Result<Self>`
    2. 移除方法内部的 `let mut backend = ProbeRsBackend::new();` 和 `backend.attach(chip)?;` — 改为直接使用传入的 backend：`backend.attach(chip)?;`
    3. 将字段赋值从 `backend: Box::new(backend)` 改为 `backend,`
    4. 删除 `use crate::probe::probe_rs::ProbeRsBackend;` 导入（不再需要）
    5. 保留 `Session::new(chip_name: String)`（标记 `#[deprecated]` 不变）
    6. 保留 `impl Default for Session`（不变）
  - **本地验证命令**: `cargo check`（会揭示所有调用 `Session::attach` 的地方需要同步更新）
  - **当前状态**: `completed`

- [x] **Task 2.2: 注册 `json_session` 模块**
  - **受影响文件**: `[src/cli/mod.rs](../../src/cli/mod.rs)`
  - **实施计划**:
    1. 在 `pub mod init;` 之后添加一行 `pub mod json_session;`
  - **本地验证命令**: `cargo check`（会提示尚未实现的模块，正常）
  - **当前状态**: `completed`

---

## 三、 CLI 启动集成 — handle() 改造 (Phase 2)

- [x] **Task 3.1: 在 `debug.rs` 中新增 `create_debug_backend()` 工厂函数**
  - **受影响文件**: `[src/cli/debug.rs](../../src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 新增 `use crate::probe::openocd::OpenOcdBackend;` 导入
    2. 新增函数：`fn create_debug_backend(backend_arg: &Option<String>) -> anyhow::Result<Box<dyn DebugProbe>>`
    3. 函数体：`let backend_type = backend_arg.as_deref().unwrap_or("probe-rs");` → match `"probe-rs"` / `"openocd"` / 其他
    4. probe-rs 分支：`Ok(Box::new(ProbeRsBackend::new()))`
    5. openocd 分支：先尝试 `.debugger/openocd.cfg`，如果存在则用该路径创建 `OpenOcdBackend`，否则报错
    6. 未知后端：`anyhow::bail!("unknown backend '{}'. Supported: probe-rs, openocd", backend_type)`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 3.2: 新增 `resolve_chip_and_flash_opts()` 辅助函数**
  - **受影响文件**: `[src/cli/debug.rs](../../src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 新增函数：`fn resolve_chip_and_flash_opts(chip_arg: &Option<String>) -> anyhow::Result<(ChipConfig, FlashOpts)>`
    2. 复用现有 `resolve_chip_for_debug()` 获取 `ChipConfig`
    3. 构造 `FlashOpts`（与 `flash.rs` 中的 `resolve_chip_config` 类似）
    4. 引入 `use crate::config::FlashOpts;`（如果未引入）
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 3.3: 改造 `handle()` 函数 — 补齐启动流程**
  - **受影响文件**: `[src/cli/debug.rs](../../src/cli/debug.rs)`
  - **函数级实施计划**:
    1. 将现有 `let chip = resolve_chip_for_debug(&args.chip)?;` 替换为 `let (chip, flash_opts) = resolve_chip_and_flash_opts(&args.chip)?;`
    2. 将现有 `let session = Session::attach(&chip)?;` 替换为两步：
       - `let mut backend = create_debug_backend(&args.backend)?;`
       - `let mut session = Session::attach(&chip, backend)?;`
    3. 在 attach 后插入烧录步骤：`if !args.no_flash { session.backend.flash(&args.elf, &flash_opts)?; }`
       - 注意：`flash()` 需要 `&mut backend`，而 backend 已被 Session 拥有。需要临时借出或修改 Session 暴露 `backend` 字段（已经 pub）。
       - 改为：`if !args.no_flash { session.backend.flash(&args.elf, &flash_opts)?; }`
    4. 遍历 break_at：`for addr_str in &args.break_at { let addr = parse_u32(addr_str)?; session.backend.set_breakpoint(addr, None)?; }`
    5. 处理 halt-on-start / continue_：`if !args.halt_on_start && args.continue_ { session.backend.resume(None)?; session.state = SessionState::Running; }`
    6. 路由到对应界面：`if args.json { ... JsonSession::new(session).run()? } else { ... DebugRepl::new(session).run()? }`
    7. 删除 `// TODO: Round 2 — 处理以下参数` 注释块
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 3.4: 调整 `DebugRepl::new()` 和 `Session::attach` 的调用关系以适配新签名**
  - **受影响文件**: `[src/cli/debug.rs](../../src/cli/debug.rs)`
  - **实施计划**:
    1. 确保 `handle()` 中的 `Session::attach(&chip, backend)` 传入了正确的 backend
    2. 确认 `DebugRepl::new(session)` 不改动 — `DebugRepl` 结构体不变
  - **注意**: `DebugRepl::new()` 的 `session: Session` 参数不变，不影响
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

---

## 四、 Agent JSON-Lines 模式 — JsonSession (Phase 3 + Phase 4)

- [x] **Task 4.1: 创建 `src/cli/json_session.rs` — 协议类型定义**
  - **受影响文件**: `[src/cli/json_session.rs](../../src/cli/json_session.rs)`
  - **函数级实施计划**:
    1. 新建文件，模块声明：`use` std / serde / serde_json / crate 相关
    2. 定义 `JsonRequest`：`#[derive(Deserialize)] struct JsonRequest { cmd: String, args: HashMap<String, Value>, id: u64 }`
    3. 定义 `JsonError`：`#[derive(Serialize)] struct JsonError { code: String, message: String }`
    4. 定义 `JsonResponse`：`#[derive(Serialize)] struct JsonResponse { id: u64, status: String, data: Option<Value>, error: Option<JsonError> }`
    5. 定义 `JsonEvent`：`#[derive(Serialize)] struct JsonEvent { event: String, data: Value }`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 4.2: 实现 `JsonSession` 结构体和 `run()` 主循环**
  - **受影响文件**: `[src/cli/json_session.rs](../../src/cli/json_session.rs)`
  - **函数级实施计划**:
    1. `pub struct JsonSession { session: Session }`
    2. `pub fn new(session: Session) -> Self { Self { session } }`
    3. `pub fn run(&mut self) -> anyhow::Result<()>` 主循环：
       ```
       loop {
           // 事件检测
           if self.session.state == SessionState::Running && self.try_check_halted() {
               // 推送 halted 事件
           }
           match Self::read_request() {
               Some(req) => {
                   let handled = self.handle_request(req);
                   if handled { break; }  // quit
               }
               None => continue,
           }
       }
       self.session.detach()?;
       ```
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 4.3: 实现 `read_request()` 和 `send_response()` / `send_event()`**
  - **受影响文件**: `[src/cli/json_session.rs](../../src/cli/json_session.rs)`
  - **函数级实施计划**:
    1. `fn read_request() -> Option<JsonRequest>`：从 stdin 读取一行 → `serde_json::from_str` → 解析失败时 send E_PARAM 并返回 None
    2. `fn send_response(id: u64, status: &str, data: Option<Value>, error: Option<JsonError>)`：序列化为 JSON → println! 到 stdout
    3. `fn send_event(event: &str, data: Value)`：序列化 `JsonEvent` → println! 到 stdout
    4. 所有 stdio 操作使用 `std::io::stdin().lines()` 和 `println!`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 4.4: 实现 `json_to_command()` — JSON 请求到 `Command` 映射**
  - **受影响文件**: `[src/cli/json_session.rs](../../src/cli/json_session.rs)`
  - **函数级实施计划**:
    1. 新增函数 `fn json_to_command(req: &JsonRequest) -> Result<Command, JsonResponse>`
    2. 对 `req.cmd` 做 match：
       - `"halt"` / `"resume"` / `"step"` / `"regs"` / `"status"` / `"help"` / `"quit"` — 无参数命令，直接映射
       - `"break"` — 从 `req.args["addr"]` 提取 u64 值，转为 u32，调用 `Command::Break { addr }`
       - `"mem"` — 从 `req.args["addr"]` 和 `req.args["len"]` 提取，调用 `Command::Mem { addr, len }`
       - `"schema"` — 特殊处理：直接返回 schema 响应
       - 未知命令 → `Err(JsonResponse { status: "error", error: Some(JsonError { code: "E_PARAM".into(), message: format!("unknown command '{}'", req.cmd) }), .. })`
    3. 参数缺失 → `E_PARAM`
    4. 参数类型错误 → `E_PARAM`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 4.5: 实现 `Schema` 生成函数**
  - **受影响文件**: `[src/cli/json_session.rs](../../src/cli/json_session.rs)`
  - **函数级实施计划**:
    1. 定义 `CommandMeta` 结构体（serde Serialize）：name / description / args / valid_states
    2. 定义 `SchemaData` 结构体（serde Serialize）：commands / error_codes
    3. 实现 `fn generate_schema() -> Value`：手写一个 const 数组或函数返回所有命令的元数据
    4. 包含 10 个命令的元数据（halt / resume / step / break / regs / mem / status / help / quit / schema）
    5. 包含 12 个错误码映射（从 `McuBridgeError` 的 code() 方法获取）
    6. schema 响应格式：`{"id":0,"status":"ok","data":{commands:[...],error_codes:{...}}}`
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

- [x] **Task 4.6: 实现事件检测方法 `try_check_halted()`**
  - **受影响文件**: `[src/cli/json_session.rs](../../src/cli/json_session.rs)`
  - **函数级实施计划**:
    1. `fn try_check_halted(&mut self) -> bool`：尝试调用 `self.session.backend.is_halted(Some(self.session.backend.active_core()))`
    2. 如果返回 true（目标已 halt）：
       - `self.session.state = SessionState::Halted;`
       - 读取 PC：`self.session.backend.read_regs(None)` 获取 `pc` 值
       - `send_event("halted", json!({"pc": pc, "core": active_core}))`
       - 返回 true
    3. 如果 `is_halted` 调用失败（探针断连），返回 false（主循环会优雅处理）
    4. 注意：`is_halted()` 当前在 `ProbeRsBackend` 中是 stub（返回 false）。但这不影响架构——事件检测逻辑已就位，待 P2 实现真实的 `is_halted` 后自动生效。
  - **本地验证命令**: `cargo check`
  - **当前状态**: `completed`

---

## 五、 测试编写 (Test Coverage)

- [x] **Task 5.1: 更新 `session.rs` 测试 — 适配新 `attach()` 签名**
  - **受影响文件**: `[src/session.rs](../../src/session.rs)`（测试模块）
  - **实施计划**:
    1. 如果 `session.rs` 的 `#[cfg(test)]` 模块中有测试直接调用 `Session::attach(&chip)`，改为 `Session::attach(&chip, backend)`，传入 mock backend
    2. 新增测试 `test_session_attach_with_backend`：创建 mock 后端 → `Session::attach(&chip, mock_backend)` → 验证状态正确
  - **注意**: `Session` 当前没有 `#[cfg(test)]` 模块（测试在 `debug.rs` 中）。可能不需要改动。
  - **确认**: 检查 `session.rs` 是否有测试代码
  - **本地验证命令**: `cargo test -- --skip test_attach_without_hardware`
  - **当前状态**: `completed`

- [x] **Task 5.2: 新增 JSON-Lines 协议测试**
  - **受影响文件**: `[src/cli/json_session.rs](../../src/cli/json_session.rs)`（测试模块）
  - **函数级实施计划**:
    1. 在文件末尾添加 `#[cfg(test)] mod tests { ... }`
    2. 测试 1 `test_json_to_command_halt`：构造 `JsonRequest { cmd: "halt", args: {}, id: 1 }` → 验证映射到 `Command::Halt`
    3. 测试 2 `test_json_to_command_break`：构造带 addr=0x8000100 的请求 → 验证 `Command::Break { addr: 0x08000100 }`
    4. 测试 3 `test_json_to_command_unknown`：未知 cmd → Err 含 E_PARAM
    5. 测试 4 `test_json_to_command_missing_arg`：break 无 addr → Err 含 E_PARAM
    6. 测试 5 `test_schema_has_all_commands`：`generate_schema()` 返回的 commands 数组包含 10 个条目
    7. 测试 6 `test_schema_has_error_codes`：schema 包含全部 12 个错误码
    8. 测试 7 `test_event_halted_format`：构造 `JsonEvent` → 序列化为 `{"event":"halted","data":{"pc":134219776}}`
  - **本地验证命令**: `cargo test -- --skip test_attach_without_hardware`
  - **当前状态**: `completed`

- [x] **Task 5.3: 新增 CLI 启动流程测试**
  - **受影响文件**: `[src/cli/debug.rs](../../src/cli/debug.rs)`（现有测试模块）
  - **函数级实施计划**:
    1. 在现有 `mod tests` 中添加以下测试：
    2. `test_handle_backend_probe_rs_default`：`create_debug_backend(&None)` 返回 `Ok(Box<ProbeRsBackend>)`
    3. `test_handle_backend_openocd_no_cfg`：`create_debug_backend(&Some("openocd".into()))` 在无 `.debugger/openocd.cfg` 时报错
    4. `test_handle_backend_unknown`：`create_debug_backend(&Some("invalid".into()))` 报 "unknown backend"
  - **本地验证命令**: `cargo test -- --skip test_attach_without_hardware`
  - **当前状态**: `completed`

---

## 六、 全局集成检验与 DoD 验证 (Whole Loop Verification)

- [x] **Task 6.1: 全量测试 + fmt + clippy 验证**
  - **描述**: 运行完整功能链，断言满足 DoD 指标。
  - **执行命令**:
    1. `cargo test -- --skip test_attach_without_hardware` — 全部通过
    2. `cargo fmt --all -- --check` — 零差异
    3. `cargo clippy --all-targets --all-features -- -D warnings` — 零警告
    4. `cargo check` — 编译通过
  - **当前状态**: `completed`

- [x] **Task 6.2: context.md 与归档**
  - **描述**: 如有新知识反哺 context.md，执行 Archive-and-Summary。
  - **实施计划**:
    1. 评估是否需要更新 context.md（新术语/决策？— JsonSession、JSON-Lines 事件推送、混合模式协议已是共识）
    2. 执行 `archive-and-summary debug-round2` 归档
  - **当前状态**: `completed`
