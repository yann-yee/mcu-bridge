# 需求规格说明书：Debug Round 2 — CLI 启动集成 + Agent JSON-Lines 模式

> ⓘ 本文档已经过 "Understanding (Grill Me)" 技能对齐，记录了 11 轮决策的完整共识树。本文件已于 [user_plan/debug-round2/debug-round2.md](debug-round2.md) 归档。实现该特性的后续 Agent 必须全盘以此文档为验收终点。

---

## 一、 功能概览与基本愿景 (User Story & Goal)

- **新功能背景**: `mcu-bridge` 已有 Human REPL 交互调试（Round 1 — DebugRepl，9 条命令，46 个测试）。但 `handle()` 中仍有 12 个 CLI 参数是 `// TODO: Round 2` 空壳状态，且缺少 Agent 模式（`--json`）——而 Agent 模式是 `mcu-bridge` 产品的核心差异化价值（"面向 AI Agent 的嵌入式调试中间件"）。本需求补齐这两块拼图。
- **用户故事 (User Story)**:
  - 作为一名嵌入式开发者，我想要 `mcu-bridge debug --break-at 0x08000100,0x08000200 --continue_ --elf fw.elf --chip STM32F407VG` 一条命令完成设断点+全速运行，以便自动化启动调试工作流。
  - 作为一名 AI Agent（首要受众），我想要 `mcu-bridge debug --elf fw.elf --chip STM32F407VG --json` 启动 JSON-Lines 协议会话，以便通过 stdin/stdout 结构化协议完成"烧录→设断点→运行→读取寄存器→停止"的全自动调试闭环。
- **关联已有的技术链**:
  - `src/session.rs` — `Session::attach()` 当前硬编码 `ProbeRsBackend`，需改为外部注入
  - `src/cli/debug.rs` — `DebugRepl` + `Command` enum + `execute()` dispatch 已完成，`handle()` 中 12 个 TODO 参数待实现
  - `src/cli/mod.rs` — `Commands::Debug` 15 个字段已全部定义，无需改动
  - `src/cli/flash.rs` — `create_backend()` 工厂不应被复用（各自独立），但可作为参考
  - `src/probe/mod.rs` — `DebugProbe` trait 稳定，18 个方法签名冻结
  - `src/probe/probe_rs.rs` — `attach/flash/halt/resume/step/breakpoint/mem/regs` 已全部实现
  - `src/probe/openocd.rs` — `attach`/`flash`/`resume`/`detach` 已实现（OpenOCD 作为 `--backend` 选项）
  - `serde_json` — 已在 Cargo.toml 依赖中（被 probe-rs 传递依赖引入）

### 本轮不做

| 排除项 | 说明 |
|--------|------|
| ❌ DebugBuffer 定时采样 + ring buffer | 独立 P1 特性，Round 3 |
| ❌ `--watch` 变量观测 | 需要 DebugBuffer，Round 3 |
| ❌ `--sampling-interval` 采样周期配置 | 需要 DebugBuffer，Round 3 |
| ❌ `--serial-port` 串口端口配置 | LogChannel 特性的一部分 |
| ❌ `--enable-flash-bp` Flash 断点 | P3 特性，参数仅解析不实现 |
| ❌ `--verify` 烧录校验功能 | P0 flash 子命令的优化项 |
| ❌ `--config` 自定义配置路径 | 参数仅解析不实现 |
| ❌ `no_flash` 在 REPL 内烧录 | Standalone `mcu-bridge flash` 已独立存在 |

---

## 二、 极限对齐的业务流程 (Consensus Flow & Sequence)

### 共识决策树

| 决策节点 | 选定方案 | 替代方案否决理由 |
|---------|---------|----------------|
| Q1: 交付范围 | `[B]` CLI 集成 + Agent JSON-Lines | A(仅 CLI) 失去核心差异价值；C(+DebugBuffer) 风险太大 |
| Q2: 协议格式 | `[B]` 带 id 的 JSON (非极简、非 JSON-RPC) | A(无id) 丢失请求追踪；C(JSON-RPC) 错误码不兼容 |
| Q3: 事件模型 | `[C]` 混合模式 (请求响应 + 事件行推送) | A(纯轮询) 延迟高；B(纯推送) 存在竞态 |
| Q4: 后端工厂 | `[B]` debug 独立创建逻辑 | A(复用flash.rs) 耦合不必要配置；C(Session接管) 职责错位 |
| Q5: 烧录时机 | `[C]` 同一 session 先 flash 再调试 | A(两步) 增加交互轮次；B(两次 attach) 多余 detach |
| Q6: Session 后端注入 | `[A]` attach 新增 backend 参数 | B(双构造) 选择困难；C(match) Session 感知所有后端 |
| Q7: Schema 格式 | `[B]` 结构化命令元数据 JSON | A(文本) Agent 硬解析；C(TypeScript) 语言绑定 |
| Q8: halt-on-start vs continue_ | `[C]` halt-on-start 优先；continue_ 为 fallback | A(continue_) 忽略 halt-on-start；B(冲突报错) 过渡设计 |
| Q9: --verify 功能 | `[A]` 保留参数不实现 (本轮不实现 flash verify) | 需要改 probe_rs.rs 且非本轮核心目标 |
| Q10: 模块组织 | `[B]` 新建 src/cli/json_session.rs | A(全塞debug.rs) 膨胀到 700+ 行 |

### 1. Human REPL 标准流 (Happy Path)

1. 用户执行 `mcu-bridge debug --elf fw.elf --chip STM32F407VG --break-at 0x08000100,0x08000200 --continue_`
2. `handle()` 解析 CLI 参数，校验 ELF 存在 → 创建后端 → `Session::attach(chip, backend)`
3. if `!no_flash`: `backend.flash(&elf, &opts)` — 烧录固件（默认开启）
4. 遍历 `break_at` 列表：对每个地址调用 `backend.set_breakpoint(addr, None)`
5. —halt-on-start 优先—: 默认 halt 状态（attach 后即 halt）；若 `!halt_on_start && continue_` → `backend.resume(None)` 进入 Running
6. 进入 REPL 循环 `(mcu) > `

### 2. Agent JSON-Lines 标准流 (Happy Path)

1. Agent 启动子进程：`mcu-bridge debug --elf fw.elf --chip STM32F407VG --json`
2. Agent 发送 `{"cmd":"schema","id":0}` → 收到完整命令元数据
3. Agent 根据 schema 发送 `{"cmd":"break","args":{"addr":134219776},"id":1}` → 收到 `{"id":1,"status":"ok","data":{"bp_id":0}}`
4. Agent 发送 `{"cmd":"resume","id":2}` → 收到 `{"id":2,"status":"ok"}`
5. 断点命中时，stdout 出现事件行：`{"event":"halted","data":{"reason":"breakpoint","pc":134219776,"bp_id":0}}`
6. Agent 可继续发送命令进行后续调试

### 3. JSON-Lines 协议规范

#### 请求格式

```json
{"cmd":"<command>","args":{...},"id":<number>}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `cmd` | string | 是 | 命令名（与 Human REPL 命令名一致） |
| `args` | object | 否 | 命令参数（key-value 形式） |
| `id` | number | 是 | 请求序列号，用于配对响应 |

#### 响应格式

```json
{"id":<number>,"status":"ok|error","data":{...},"error":{...}}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | number | 是 | 对应请求的 id |
| `status` | string | 是 | `"ok"` 或 `"error"` |
| `data` | object | 否 | 成功时的返回数据 |
| `error` | object | 否 | 失败时的错误信息 `{"code":"E_XXX","message":"..."}` |

#### 事件格式

```json
{"event":"<event_name>","data":{...}}
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `event` | string | 是 | 事件名（当前唯一事件：`halted`） |
| `data` | object | 是 | 事件参数 |

#### schema 响应结构

```json
{
  "id": 0,
  "status": "ok",
  "data": {
    "commands": [
      {
        "name": "halt",
        "description": "Pause target execution",
        "valid_states": ["Running"]
      },
      {
        "name": "break",
        "description": "Set hardware breakpoint",
        "args": [
          {"name": "addr", "type": "u32", "required": true, "format": "hex|dec"}
        ],
        "valid_states": ["Halted"]
      }
    ],
    "error_codes": {
      "E_STATE": "command not valid in current target state",
      "E_PARAM": "invalid or missing parameter",
      ...
    }
  }
}
```

#### 命令到参数的映射

| JSON `cmd` | `args` 字段 | 映射为 Command |
|-----------|------------|---------------|
| `halt` | — | `Command::Halt` |
| `resume` | — | `Command::Resume` |
| `step` | — | `Command::Step` |
| `break` | `{"addr": N}` | `Command::Break { addr }` |
| `regs` | — | `Command::Regs` |
| `mem` | `{"addr": N, "len": N}` | `Command::Mem { addr, len }` |
| `status` | — | `Command::Status` |
| `help` | — | `Command::Help` |
| `quit` | — | `Command::Quit` |
| `schema` | — | 特殊处理，返回元数据 |

### 4. 异常与阻断流

| 失败场景 | 可见消息 | 系统行为 |
|---------|---------|---------|
| ELF 文件不存在 | `"ELF file not found: {path}"` | 报错退出 (exit=1) |
| 芯片未知 | `"unknown chip '{name}'"` | 报错退出 |
| 后端类型未知 | `"unknown backend '{name}'. Supported: probe-rs, openocd"` | 报错退出 |
| JSON 解析失败 | `{"id":null,"status":"error","error":{"code":"E_PARAM","message":"invalid JSON: ..."}}` | 继续读取下一行 |
| 未知 JSON 命令 | `{"id":N,"status":"error","error":{"code":"E_PARAM","message":"unknown command 'xxx'"}}` | 继续循环 |
| 命令参数缺失 | `{"id":N,"status":"error","error":{"code":"E_PARAM","message":"missing required arg: addr"}}` | 继续循环 |
| 状态守卫拦截 | `{"id":N,"status":"error","error":{"code":"E_STATE","message":"command 'step' not valid in Running state"}}` | 继续循环 |
| 探针操作失败 | `{"id":N,"status":"error","error":{"code":"E_BACKEND","message":"halt failed: ..."}}` | 继续循环 |
| 空行 / 空白输入 | — | 静默忽略 |
| 探针意外断连 | Human: `"[ERROR] probe disconnected"`; Agent: `{"event":"error","data":{"code":"E_PROBE_LOST","message":"..."}}` 然后 exit | 优雅退出 |

---

## 三、 架构设计方案 (Architecture Design)

### 3.1 `Session::attach()` 改造

```rust
// src/session.rs — 改动
impl Session {
    /// 连接探针并创建会话（初始状态 Halted）。
    /// 调用方负责创建并传入 backend（可注入 mock 便于测试）。
    pub fn attach(chip: &ChipConfig, backend: Box<dyn DebugProbe>) -> anyhow::Result<Self> {
        backend.attach(chip)?;
        let core_count = backend.core_count();
        info!("session attached to {} ({} core(s))", chip.name, core_count);
        Ok(Self {
            state: SessionState::Halted,
            chip_name: chip.name.clone(),
            core_count,
            pc: None,
            bp_count: 0,
            watch_count: 0,
            backend,
        })
    }
}
```

**影响**: Round 1 中 `handle()` 调用 `Session::attach(&chip)` 的方式需要更新为 `Session::attach(&chip, backend)`。所有 `Session::attach` 的调用点（包括 `session.rs` 中的测试辅助代码）都需要更新。

### 3.2 `handle()` 启动流程（改造后）

```
handle(args):
    1. 校验 ELF 文件存在
    2. resolve_chip_and_flash_opts() — 解析芯片配置 + FlashOpts
    3. create_debug_backend(args) — 根据 --backend 创建后端
    4. let session = Session::attach(&chip, backend)?
    5. if !no_flash: backend.flash(&elf, &flash_opts)?
    6. for addr in break_at: backend.set_breakpoint(addr, None)?
    7. if !halt_on_start && continue_: backend.resume(None); state = Running
    8. if json: JsonSession::new(session).run()?
       else: DebugRepl::new(session).run()?
```

### 3.3 `JsonSession` 结构体

```rust
// src/cli/json_session.rs (新建)
pub struct JsonSession {
    session: Session,
}

impl JsonSession {
    pub fn new(session: Session) -> Self { ... }

    /// 进入 JSON-Lines 协议循环
    pub fn run(&mut self) -> anyhow::Result<()> { ... }

    // ── 内部方法 ──

    /// 读取一行 JSON 请求
    fn read_request(&mut self) -> Option<JsonRequest> { ... }

    /// 将 JSON 请求映射到 Command 并执行
    fn handle_request(&mut self, req: JsonRequest) { ... }

    /// 向 stdout 写一行 JSON 响应
    fn send_response(&self, resp: &JsonResponse) { ... }

    /// 检测异步事件并推送事件行
    fn check_events(&mut self) { ... }
}

/// JSON 请求
#[derive(Deserialize)]
struct JsonRequest {
    cmd: String,
    #[serde(default)]
    args: HashMap<String, serde_json::Value>,
    id: u64,
}

/// JSON 响应
#[derive(Serialize)]
struct JsonResponse {
    id: u64,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonError>,
}
```

### 3.4 事件检测机制

`JsonSession::run()` 主循环结构：

```
loop {
    // 1. 检测异步事件（仅在 Running 态有意义）
    if session.state == Running && backend.is_halted(Some(active_core)) {
        session.state = Halted;
        send_event("halted", {"reason": "breakpoint", "pc": pc});
    }

    // 2. 读取请求（阻塞）
    match read_request() {
        Some(req) => handle_request(req),
        None if req is quit => break,
        None => continue,
    }
}
session.detach()?;
```

**注意**: 事件检测是"机会主义的"——每次循环先检测状态变化再等待输入。

### 3.5 `create_debug_backend()` 工厂

```rust
// src/cli/debug.rs — 新增
fn create_debug_backend(backend_arg: &Option<String>) -> anyhow::Result<Box<dyn DebugProbe>> {
    let backend_type = backend_arg.as_deref().unwrap_or("probe-rs");
    match backend_type.to_ascii_lowercase().as_str() {
        "probe-rs" => Ok(Box::new(ProbeRsBackend::new())),
        "openocd" => {
            let cfg = resolve_openocd_cfg()?;
            Ok(Box::new(OpenOcdBackend::new(Some(cfg))))
        }
        _ => anyhow::bail!("unknown backend '{backend_type}'. Supported: probe-rs, openocd"),
    }
}
```

---

## 四、 受影响文件清单 (Affect Map)

> ⚠ 此清单仅列出受影响的文件，**不是执行顺序**。执行顺序由后续 `/code-spec` 生成的 `task.md` 定义。

| 文件 | 改动类型 | 说明 |
|------|---------|------|
| `src/session.rs` | 🟡 修改 | `Session::attach()` 签名改为 `attach(chip, backend)` |
| `src/cli/debug.rs` | 🟢 实现 | `handle()` 补齐 12 个 TODO 参数；新增 `create_debug_backend()`；烧录/断点/continue 逻辑 |
| `src/cli/json_session.rs` | 🟢 新建 | JsonSession 结构体 + 协议循环 + schema 生成 + 事件推送 |
| `src/cli/mod.rs` | 🟢 注册 | `pub mod json_session;` |
| `src/probe/mod.rs` | ⬜ 不动 | `DebugProbe` trait 签名冻结 |
| `src/probe/probe_rs.rs` | ⬜ 不动 | 所有方法已实现 |
| `src/probe/openocd.rs` | ⬜ 不动 | 已有方法足够 |
| `src/main.rs` | ⬜ 不动 | `Commands::Debug` 路由已存在 |
| `Cargo.toml` | ⬜ 不动 | 不新增外部依赖 |

---

## 五、 测试策略 (Test Strategy)

### 单元测试

| 测试类别 | 测试函数 | 说明 |
|---------|---------|------|
| **Session 签名** | `test_session_attach_with_backend` | 传入 mock backend，验证 attach 调用传递 |
| | `test_session_attach_detach` | attach → detach 顺序正确 |
| **CLI 启动流程** | `test_handle_break_at` | 传入 break-at 列表，验证 breakpoint 被设置 |
| | `test_handle_continue` | --continue_ → resume 被调用 |
| | `test_handle_no_flash` | --no-flash → flash 不被调用 |
| | `test_handle_halt_on_start` | --halt-on-start → 不 resume |
| | `test_handle_backend_probe_rs` | 默认 → probe-rs backend |
| | `test_handle_backend_openocd` | --backend openocd → OpenOcdBackend |
| | `test_handle_backend_unknown` | --backend invalid → 报错 |
| **Cmd 映射** | `test_json_to_command_halt` | 映射到 Command::Halt |
| | `test_json_to_command_break` | 映射到 Command::Break |
| | `test_json_to_command_unknown` | 未知 cmd → Err |
| **Schema** | `test_schema_response_has_all_commands` | schema 返回包含 10 个命令 |
| | `test_schema_response_has_error_codes` | schema 包含 12 个错误码 |
| **JSON-Lines 协议** | `test_json_session_req_resp_pair` | 请求 → 对应 id 的响应 |
| | `test_json_session_state_guard` | Running 态发 step → E_STATE |
| | `test_json_session_event_halted` | 断点命中 → halted 事件 |
| | `test_json_session_invalid_json` | 无效 JSON → E_PARAM |
| **回归** | 已有 46 个测试不变 | 全部通过 |

### Mock 策略

- `Session::attach()` 接受外部 `Box<dyn DebugProbe>`，测试时注入 mock backend
- JSON-Lines 测试：用 `Cursor` 模拟 stdin，用 `Vec<u8>` 模拟 stdout

---

## 六、 验收断言与 Definition of Done

- [ ] **1. CLI 参数全可达**: `--break-at`、`--continue_`、`--halt-on-start`、`--no-flash`、`--backend`、`--json` 各自触发对应行为，不 panic、不 todo!()
- [ ] **2. `Session::attach(chip, backend)` 新签名**: 调用方传入 backend，而非硬编码 probe-rs
- [ ] **3. Agent JSON-Lines 模式启动**: `--json` 启动后无提示符，等待 stdin JSON
- [ ] **4. JSON schema 自发现**: `{"cmd":"schema","id":0}` 返回全部命令元数据 + 12 个错误码
- [ ] **5. 9 条 JSON 命令可执行**: halt/resume/step/break/regs/mem/status/help/quit → 响应带正确 id + status
- [ ] **6. 事件推送**: Running→Halted 时 stdout 出现 `{"event":"halted",...}` 事件行
- [ ] **7. 状态守卫**: Running 态发 step → `"status":"error","error":{"code":"E_STATE",...}`
- [ ] **8. 已有测试全绿**: 原有 46 个测试不变
- [ ] **9. 零残留 todo!()**: `src/cli/debug.rs` 中无 todo!() 残留
- [ ] **10. 格式合规**: `cargo fmt --all -- --check` 零差异；`cargo clippy --all-targets --all-features -- -D warnings` 零警告
