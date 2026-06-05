//! JSON-Lines Agent 模式 — stdin/stdout 结构化调试协议。
//!
//! 设计文档 §4.2：Agent JSON-Lines 模式
//!   stdin → 每行一个 JSON-RPC 式请求 `{"cmd":"<name>","args":{...},"id":<num>}`
//!   stdout → 每行一个 JSON 响应 `{"id":<num>,"status":"ok|error","data":{...},"error":{...}}`
//!   异步事件推送 `{"event":"halted","data":{"pc":N,"core":N}}`

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, mpsc};

use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};

use crate::buffer::serial::LogEvent;
use crate::buffer::{DebugBuffer, LogBuffer, LogEntry, Sampler};
use crate::cli::debug::Command;
use crate::session::{Session, SessionState};

// ── JSON-Lines 协议类型 ──

/// JSON 请求（来自 stdin）
#[derive(Debug, Deserialize)]
pub struct JsonRequest {
    /// 命令名
    pub cmd: String,
    /// 命令参数（key-value）
    #[serde(default)]
    pub args: HashMap<String, Value>,
    /// 请求序列号
    pub id: u64,
}

/// JSON 错误
#[derive(Debug, Serialize)]
pub struct JsonError {
    /// 错误码（如 E_PARAM、E_STATE）
    pub code: String,
    /// 人类可读的错误描述
    pub message: String,
}

/// JSON 响应（写入 stdout）
#[derive(Debug, Serialize)]
pub struct JsonResponse {
    pub id: u64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonError>,
}

/// JSON 事件推送（写入 stdout，独立于请求-响应配对）
#[derive(Debug, Serialize)]
pub struct JsonEvent {
    pub event: String,
    pub data: Value,
}

// ── Schema 命令元数据 ──

/// schema 中单条命令的描述
#[derive(Debug, Serialize)]
pub struct CommandMeta {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<ArgMeta>>,
    pub valid_states: Vec<String>,
}

/// 命令参数元数据
#[derive(Debug, Serialize)]
pub struct ArgMeta {
    pub name: String,
    #[serde(rename = "type")]
    pub arg_type: String,
    pub required: bool,
}

/// schema 返回的顶层数据
#[derive(Debug, Serialize)]
pub struct SchemaData {
    pub commands: Vec<CommandMeta>,
    pub error_codes: HashMap<String, String>,
}

/// 生成 schema 响应数据
fn generate_schema() -> SchemaData {
    let commands = vec![
        CommandMeta {
            name: "halt".into(),
            description: "Pause target execution".into(),
            args: None,
            valid_states: vec!["Running".into()],
        },
        CommandMeta {
            name: "resume".into(),
            description: "Resume target execution".into(),
            args: None,
            valid_states: vec!["Halted".into()],
        },
        CommandMeta {
            name: "step".into(),
            description: "Single-step (halted)".into(),
            args: None,
            valid_states: vec!["Halted".into()],
        },
        CommandMeta {
            name: "break".into(),
            description: "Set hardware breakpoint".into(),
            args: Some(vec![ArgMeta {
                name: "addr".into(),
                arg_type: "u32".into(),
                required: true,
            }]),
            valid_states: vec!["Halted".into()],
        },
        CommandMeta {
            name: "regs".into(),
            description: "Show core registers".into(),
            args: None,
            valid_states: vec!["Halted".into()],
        },
        CommandMeta {
            name: "mem".into(),
            description: "Read memory".into(),
            args: Some(vec![
                ArgMeta {
                    name: "addr".into(),
                    arg_type: "u32".into(),
                    required: true,
                },
                ArgMeta {
                    name: "len".into(),
                    arg_type: "u32".into(),
                    required: true,
                },
            ]),
            valid_states: vec!["Halted".into()],
        },
        CommandMeta {
            name: "status".into(),
            description: "Show session status".into(),
            args: None,
            valid_states: vec![],
        },
        CommandMeta {
            name: "help".into(),
            description: "Show available commands".into(),
            args: None,
            valid_states: vec![],
        },
        CommandMeta {
            name: "quit".into(),
            description: "Exit debug session".into(),
            args: None,
            valid_states: vec![],
        },
        CommandMeta {
            name: "schema".into(),
            description: "Get protocol self-description".into(),
            args: None,
            valid_states: vec![],
        },
        CommandMeta {
            name: "watch".into(),
            description: "Add a memory watch target".into(),
            args: Some(vec![
                ArgMeta {
                    name: "addr".into(),
                    arg_type: "u32".into(),
                    required: true,
                },
                ArgMeta {
                    name: "size".into(),
                    arg_type: "u32".into(),
                    required: true,
                },
                ArgMeta {
                    name: "label".into(),
                    arg_type: "string".into(),
                    required: false,
                },
            ]),
            valid_states: vec!["Halted".into()],
        },
        CommandMeta {
            name: "buffer".into(),
            description: "Query sampling history".into(),
            args: Some(vec![
                ArgMeta {
                    name: "since".into(),
                    arg_type: "u64".into(),
                    required: false,
                },
                ArgMeta {
                    name: "watch_id".into(),
                    arg_type: "usize".into(),
                    required: false,
                },
            ]),
            valid_states: vec![],
        },
        CommandMeta {
            name: "serial".into(),
            description: "Query serial log history (read from ring buffer)".into(),
            args: Some(vec![
                ArgMeta {
                    name: "since".into(),
                    arg_type: "u64".into(),
                    required: false,
                },
                ArgMeta {
                    name: "channel".into(),
                    arg_type: "string".into(),
                    required: false,
                },
            ]),
            valid_states: vec![],
        },
    ];

    let error_codes = HashMap::from([
        (
            "E_STATE".into(),
            "command not valid in current target state".into(),
        ),
        ("E_PARAM".into(), "invalid or missing parameter".into()),
        ("E_BACKEND".into(), "backend communication failure".into()),
        (
            "E_PROBE".into(),
            "probe disconnected, recovery in progress".into(),
        ),
        (
            "E_PROBE_LOST".into(),
            "probe recovery failed, session ending".into(),
        ),
        ("E_FLASH".into(), "flash operation failed".into()),
        (
            "E_NO_DWARF".into(),
            "DWARF info needed but not available".into(),
        ),
        (
            "E_NO_SEMIHOSTING".into(),
            "operation not supported in semihosting mode".into(),
        ),
        (
            "E_FLASH_BP_DISABLED".into(),
            "flash breakpoints not enabled".into(),
        ),
        (
            "E_FLASH_BP_LIMIT".into(),
            "flash breakpoint session limit reached".into(),
        ),
        ("E_SERIAL".into(), "serial port operation failed".into()),
        ("E_INTERNAL".into(), "internal error".into()),
    ]);

    SchemaData {
        commands,
        error_codes,
    }
}

// ── JSON → Command 映射 ──

/// 将 JSON 请求映射为 Command。
///
/// 返回 `Ok(Command)` 可执行；返回 `Err(JsonResponse)` 可直接发送给客户端。
pub fn json_to_command(req: &JsonRequest) -> Result<Command, JsonResponse> {
    let err_response = |code: &str, msg: String| -> JsonResponse {
        JsonResponse {
            id: req.id,
            status: "error".into(),
            data: None,
            error: Some(JsonError {
                code: code.into(),
                message: msg,
            }),
        }
    };

    match req.cmd.as_str() {
        "halt" => Ok(Command::Halt),
        "resume" => Ok(Command::Resume),
        "step" => Ok(Command::Step),
        "break" => {
            let addr = req
                .args
                .get("addr")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| {
                    err_response(
                        "E_PARAM",
                        "missing or invalid 'addr' (hex or decimal)".into(),
                    )
                })?;
            Ok(Command::Break { addr })
        }
        "regs" => Ok(Command::Regs),
        "mem" => {
            let addr = req
                .args
                .get("addr")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| {
                    err_response(
                        "E_PARAM",
                        "missing or invalid 'addr' (hex or decimal)".into(),
                    )
                })?;
            let len = req
                .args
                .get("len")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| {
                    err_response("E_PARAM", "missing or invalid 'len' (decimal)".into())
                })?;
            Ok(Command::Mem { addr, len })
        }
        "status" => Ok(Command::Status),
        "help" => Ok(Command::Help),
        "quit" => Ok(Command::Quit),
        "schema" => unreachable!(), // handled in handle_request before json_to_command
        "watch" => {
            let addr = req
                .args
                .get("addr")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| err_response("E_PARAM", "missing or invalid 'addr'".into()))?;
            let size = req
                .args
                .get("size")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .ok_or_else(|| err_response("E_PARAM", "missing or invalid 'size'".into()))?;
            let label = req
                .args
                .get("label")
                .and_then(|v| v.as_str().map(|s| s.to_string()));
            if !matches!(size, 1 | 2 | 4 | 8) {
                return Err(err_response("E_PARAM", "size must be 1, 2, 4, or 8".into()));
            }
            Ok(Command::Watch { addr, size, label })
        }
        "buffer" => {
            let since = req.args.get("since").and_then(|v| v.as_u64());
            let watch_id = req
                .args
                .get("watch_id")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize);
            Ok(Command::Buffer { since, watch_id })
        }
        "serial" => {
            let since = req.args.get("since").and_then(|v| v.as_u64());
            let channel = req
                .args
                .get("channel")
                .and_then(|v| v.as_str().map(|s| s.to_string()));
            Ok(Command::Serial { since, channel })
        }
        unknown => Err(err_response(
            "E_PARAM",
            format!("unknown command '{}'", unknown),
        )),
    }
}

// ── JsonSession ──

/// JSON-Lines Agent 调试会话
pub struct JsonSession {
    /// 底层调试会话
    session: Session,
    /// 共享调试缓冲区
    buffer: Arc<RwLock<DebugBuffer>>,
    /// 共享日志缓冲区
    log_buffer: Arc<RwLock<LogBuffer>>,
    /// 日志事件接收端（从 SerialMonitor 线程接收）
    log_event_rx: Option<mpsc::Receiver<LogEvent>>,
    /// 采样线程句柄
    sampler_thread: Option<std::thread::JoinHandle<()>>,
    /// 采样停止信号
    sampler_stop: Option<Arc<AtomicBool>>,
    /// 采样间隔（ms）
    sampling_interval: u64,
}

impl JsonSession {
    /// 创建 JSON-Lines 会话
    pub fn new(
        session: Session,
        sampling_interval: u64,
        buffer_capacity: usize,
        log_buffer: Arc<RwLock<LogBuffer>>,
        log_event_rx: Option<mpsc::Receiver<LogEvent>>,
    ) -> Self {
        Self {
            session,
            buffer: Arc::new(RwLock::new(DebugBuffer::new(buffer_capacity))),
            log_buffer,
            log_event_rx,
            sampler_thread: None,
            sampler_stop: None,
            sampling_interval,
        }
    }

    /// 进入主协议循环
    pub fn run(&mut self) -> anyhow::Result<()> {
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break, // EOF / Ctrl+C
            };
            if line.trim().is_empty() {
                continue;
            }

            // 事件检测（仅在 Running 态有意义）
            self.check_events();

            // 日志事件推送（非阻塞检查 mpsc receiver）
            self.push_log_events();

            // 解析请求
            let req: JsonRequest = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(e) => {
                    Self::send_error(0, "E_PARAM", &format!("invalid JSON: {e}"));
                    continue;
                }
            };

            // 处理
            let is_quit = self.handle_request(req);
            if is_quit {
                break;
            }
        }
        self.stop_sampler();
        self.session.detach()?;
        Ok(())
    }

    /// 处理一条 JSON 请求。返回 true 表示应退出循环。
    fn handle_request(&mut self, req: JsonRequest) -> bool {
        // schema 特殊处理
        if req.cmd == "schema" {
            let schema = generate_schema();
            let resp = JsonResponse {
                id: req.id,
                status: "ok".into(),
                data: Some(serde_json::to_value(&schema).unwrap_or_default()),
                error: None,
            };
            Self::send_json(&resp);
            return false;
        }

        // 映射为 Command
        let cmd = match json_to_command(&req) {
            Ok(c) => c,
            Err(err_resp) => {
                Self::send_json(&err_resp);
                return false;
            }
        };

        // quit 特殊处理
        if matches!(cmd, Command::Quit) {
            return true;
        }

        // 状态守卫
        if let Some(states) = cmd.valid_states()
            && !states.contains(&self.session.state)
        {
            let resp = JsonResponse {
                id: req.id,
                status: "error".into(),
                data: None,
                error: Some(JsonError {
                    code: "E_STATE".into(),
                    message: format!(
                        "command '{}' not valid in {:?} state",
                        cmd, self.session.state
                    ),
                }),
            };
            Self::send_json(&resp);
            return false;
        }

        // 执行
        let result = self.execute_json(cmd);
        match result {
            Ok(data) => {
                let resp = JsonResponse {
                    id: req.id,
                    status: "ok".into(),
                    data,
                    error: None,
                };
                Self::send_json(&resp);
            }
            Err(e) => {
                let resp = JsonResponse {
                    id: req.id,
                    status: "error".into(),
                    data: None,
                    error: Some(JsonError {
                        code: "E_BACKEND".into(),
                        message: e.to_string(),
                    }),
                };
                Self::send_json(&resp);
            }
        }
        false
    }

    /// 执行命令并返回 JSON data
    fn execute_json(&mut self, cmd: Command) -> anyhow::Result<Option<Value>> {
        match cmd {
            Command::Halt => {
                self.stop_sampler();
                self.session
                    .backend
                    .lock()
                    .expect("backend lock")
                    .halt(None)?;
                self.session.state = SessionState::Halted;
                Ok(Some(json!({"status": "halted"})))
            }
            Command::Resume => {
                if self.sampler_thread.is_some() {
                    return Ok(Some(
                        json!({"status": "error", "error": "sampler already running, halt first"}),
                    ));
                }
                self.session
                    .backend
                    .lock()
                    .expect("backend lock")
                    .resume(None)?;
                self.session.state = SessionState::Running;
                // 如果有 watch target，启动采样线程
                let watch_count = self.buffer.read().unwrap().targets.len();
                if watch_count > 0 {
                    let backend = self.session.shared_backend();
                    let buffer = self.buffer.clone();
                    let mut sampler = Sampler::new(backend, buffer, self.sampling_interval, 0);
                    let stop_flag = sampler.stop_flag();
                    self.sampler_stop = Some(stop_flag);
                    self.sampler_thread = Some(std::thread::spawn(move || {
                        sampler.run();
                    }));
                }
                Ok(Some(
                    json!({"status": "running", "sampling": watch_count > 0, "sampling_interval_ms": self.sampling_interval}),
                ))
            }
            Command::Step => {
                self.session
                    .backend
                    .lock()
                    .expect("backend lock")
                    .step(None)?;
                let pc = self
                    .session
                    .backend
                    .lock()
                    .unwrap()
                    .read_regs(None)
                    .ok()
                    .and_then(|regs| {
                        regs.get("pc")
                            .or_else(|| regs.get("PC"))
                            .copied()
                            .map(|v| v as u32)
                    });
                self.session.state = SessionState::Halted;
                self.session.pc = pc;
                if let Some(pc) = pc {
                    Ok(Some(json!({"pc": pc})))
                } else {
                    Ok(Some(json!({"status": "stepped"})))
                }
            }
            Command::Break { addr } => {
                let id = self
                    .session
                    .backend
                    .lock()
                    .unwrap()
                    .set_breakpoint(addr, None)?;
                self.session.bp_count += 1;
                Ok(Some(json!({"bp_id": id, "addr": addr})))
            }
            Command::Regs => {
                let regs = self
                    .session
                    .backend
                    .lock()
                    .expect("backend lock")
                    .read_regs(None)?;
                let mut map = serde_json::Map::new();
                for (k, v) in &regs {
                    map.insert(k.clone(), json!(v));
                }
                Ok(Some(Value::Object(map)))
            }
            Command::Mem { addr, len } => {
                let data = self
                    .session
                    .backend
                    .lock()
                    .unwrap()
                    .read_mem(addr, len, None)?;
                Ok(Some(json!({"addr": addr, "len": len, "data": data})))
            }
            Command::Status => {
                let pc_str = self
                    .session
                    .pc
                    .map(|p| format!("0x{p:08x}"))
                    .unwrap_or_else(|| "?".into());
                Ok(Some(json!({
                    "state": format!("{:?}", self.session.state),
                    "chip": self.session.chip_name,
                    "bp_count": self.session.bp_count,
                    "pc": pc_str,
                    "core_count": self.session.core_count,
                })))
            }
            Command::Help => {
                let help = concat!(
                    "halt              Pause target execution\n",
                    "resume, go        Resume target execution\n",
                    "step, s           Single-step (halted)\n",
                    "break <addr>, b   Set hardware breakpoint (halted)\n",
                    "regs, registers   Show core registers (halted)\n",
                    "mem <addr> <len>  Read memory (halted)\n",
                    "watch <a>:<s>[:l] Add watch target (halted)\n",
                    "buffer [since] [watch_id] Show sampling history\n",
                    "status, st        Show session status\n",
                    "help, h, ?        Show this help\n",
                    "quit, exit, q     Exit debug session",
                );
                Ok(Some(json!({"help": help})))
            }
            Command::Quit => unreachable!(), // handled in handle_request
            Command::Watch { addr, size, label } => {
                let watch_id = self.buffer.write().unwrap().add_target(addr, size, label);
                self.session.watch_count = self.buffer.read().unwrap().targets.len();
                Ok(Some(
                    json!({"watch_id": watch_id, "addr": addr, "size": size}),
                ))
            }
            Command::Buffer { since, watch_id } => {
                let samples = self.buffer.read().unwrap().get_samples(watch_id, since);
                Ok(Some(
                    json!({"samples": samples, "count": samples.iter().map(|v| v.len()).sum::<usize>()}),
                ))
            }
            Command::Serial { since, channel } => {
                let log_buf = self.log_buffer.read().unwrap();
                let entries: Vec<&LogEntry> = log_buf.get_since(since.unwrap_or(0));
                let filtered: Vec<&LogEntry> = match &channel {
                    Some(ch) => entries.into_iter().filter(|e| &e.channel == ch).collect(),
                    None => entries,
                };
                Ok(Some(json!({
                    "entries": filtered.iter().map(|e| json!({
                        "sn": e.sn,
                        "tick_us": e.tick_us,
                        "channel": e.channel,
                        "data": e.data,
                    })).collect::<Vec<_>>(),
                    "count": filtered.len(),
                })))
            }
            Command::Info { .. } => {
                Ok(Some(json!({"status": "error", "error": "info not available in JSON-Lines mode"})))
            }
        }
    }

    /// 停止采样线程（最多等待 2 秒，超时则分离线程不阻塞主线程）。
    fn stop_sampler(&mut self) {
        if let Some(stop) = self.sampler_stop.take() {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(handle) = self.sampler_thread.take() {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while std::time::Instant::now() < deadline {
                if handle.is_finished() {
                    let _ = handle.join();
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            // 超时 → 分离
        }
    }

    /// 检测目标是否已 halt（Running → Halted 状态切换时推送事件）
    fn check_events(&mut self) {
        if self.session.state != SessionState::Running {
            return;
        }
        // 一次性锁定 backend，连续完成所有检测操作
        let mut guard = self.session.backend.lock().expect("backend lock poisoned");
        let active_core = guard.active_core();
        if !guard.is_halted(Some(active_core)) {
            return;
        }
        // 目标已 halt
        self.session.state = SessionState::Halted;
        drop(guard); // 释放锁，允许 stop_sampler 获取
        self.stop_sampler();
        // 重新锁 backend 读取 PC
        let pc = self
            .session
            .backend
            .lock()
            .unwrap()
            .read_regs(None)
            .ok()
            .and_then(|regs| {
                regs.get("pc")
                    .or_else(|| regs.get("PC"))
                    .copied()
                    .map(|v| v as u32)
            });
        let event = JsonEvent {
            event: "halted".into(),
            data: json!({
                "pc": pc.unwrap_or(0),
                "core": active_core,
            }),
        };
        Self::send_json(&event);
    }

    /// 从 mpsc receiver 拉取日志事件并推送到 stdout（非阻塞）。
    fn push_log_events(&mut self) {
        let rx = match self.log_event_rx.as_mut() {
            Some(r) => r,
            None => return,
        };
        // 尝试接收所有已到达的事件，但最多 10 条/轮以避免阻塞主循环
        for _ in 0..10 {
            match rx.try_recv() {
                Ok(event) => {
                    let json_event = JsonEvent {
                        event: "log".into(),
                        data: json!({
                            "channel": event.channel,
                            "data": event.data,
                        }),
                    };
                    Self::send_json(&json_event);
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    // SerialMonitor 已停止，移除 receiver
                    self.log_event_rx = None;
                    break;
                }
            }
        }
    }

    // ── I/O 辅助方法 ──

    /// 向 stdout 写入一行 JSON
    fn send_json<T: Serialize>(msg: &T) {
        match serde_json::to_string(msg) {
            Ok(json_str) => println!("{json_str}"),
            Err(e) => eprintln!("[FATAL] JSON serialization failed: {e}"),
        }
    }

    /// 快速发送错误响应
    fn send_error(id: u64, code: &str, message: &str) {
        let resp = JsonResponse {
            id,
            status: "error".into(),
            data: None,
            error: Some(JsonError {
                code: code.into(),
                message: message.into(),
            }),
        };
        Self::send_json(&resp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── json_to_command 映射测试 ──

    #[test]
    fn test_json_to_command_halt() {
        let req = JsonRequest {
            cmd: "halt".into(),
            args: HashMap::new(),
            id: 1,
        };
        assert!(matches!(json_to_command(&req).unwrap(), Command::Halt));
    }

    #[test]
    fn test_json_to_command_resume() {
        let req = JsonRequest {
            cmd: "resume".into(),
            args: HashMap::new(),
            id: 1,
        };
        assert!(matches!(json_to_command(&req).unwrap(), Command::Resume));
    }

    #[test]
    fn test_json_to_command_step() {
        let req = JsonRequest {
            cmd: "step".into(),
            args: HashMap::new(),
            id: 1,
        };
        assert!(matches!(json_to_command(&req).unwrap(), Command::Step));
    }

    #[test]
    fn test_json_to_command_break() {
        let mut args = HashMap::new();
        args.insert("addr".into(), json!(0x08000100u32));
        let req = JsonRequest {
            cmd: "break".into(),
            args,
            id: 1,
        };
        let cmd = json_to_command(&req).unwrap();
        assert_eq!(cmd, Command::Break { addr: 0x08000100 });
    }

    #[test]
    fn test_json_to_command_break_missing_addr() {
        let req = JsonRequest {
            cmd: "break".into(),
            args: HashMap::new(),
            id: 1,
        };
        let err = json_to_command(&req).unwrap_err();
        assert_eq!(err.status, "error");
        assert_eq!(err.error.as_ref().unwrap().code, "E_PARAM");
    }

    #[test]
    fn test_json_to_command_unknown() {
        let req = JsonRequest {
            cmd: "xyz".into(),
            args: HashMap::new(),
            id: 1,
        };
        let err = json_to_command(&req).unwrap_err();
        assert_eq!(err.error.as_ref().unwrap().code, "E_PARAM");
        assert!(err.error.unwrap().message.contains("unknown command"));
    }

    #[test]
    fn test_json_to_command_mem() {
        let mut args = HashMap::new();
        args.insert("addr".into(), json!(0x20000000u32));
        args.insert("len".into(), json!(16u32));
        let req = JsonRequest {
            cmd: "mem".into(),
            args,
            id: 1,
        };
        let cmd = json_to_command(&req).unwrap();
        assert_eq!(
            cmd,
            Command::Mem {
                addr: 0x20000000,
                len: 16
            }
        );
    }

    #[test]
    fn test_json_to_command_mem_missing_len() {
        let mut args = HashMap::new();
        args.insert("addr".into(), json!(0x20000000u32));
        let req = JsonRequest {
            cmd: "mem".into(),
            args,
            id: 1,
        };
        let err = json_to_command(&req).unwrap_err();
        assert_eq!(err.error.as_ref().unwrap().code, "E_PARAM");
    }

    // ── Schema 测试 ──

    #[test]
    fn test_schema_has_all_commands() {
        let schema = generate_schema();
        // 13 个命令: halt, resume, step, break, regs, mem, status, help, quit, schema, watch, buffer, serial
        assert_eq!(schema.commands.len(), 13);
    }

    #[test]
    fn test_schema_has_error_codes() {
        let schema = generate_schema();
        // 12 个错误码
        assert_eq!(schema.error_codes.len(), 12);
        assert!(schema.error_codes.contains_key("E_STATE"));
        assert!(schema.error_codes.contains_key("E_PARAM"));
        assert!(schema.error_codes.contains_key("E_BACKEND"));
        assert!(schema.error_codes.contains_key("E_PROBE_LOST"));
    }

    // ── 事件格式测试 ──

    #[test]
    fn test_event_halted_format() {
        let event = JsonEvent {
            event: "halted".into(),
            data: json!({"pc": 0x08000100u32, "core": 0}),
        };
        let json_str = serde_json::to_string(&event).unwrap();
        assert!(json_str.contains("\"event\":\"halted\""));
        assert!(json_str.contains("\"pc\":"));
    }
}
