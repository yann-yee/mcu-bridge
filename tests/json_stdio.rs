use std::io::{BufRead, BufReader, Write};
use std::ops::{Deref, DerefMut};
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

use mcu_bridge::buffer::LogBuffer;
use mcu_bridge::buffer::serial::LogEvent;
use mcu_bridge::cli::json_session::JsonSession;
use mcu_bridge::config::{ChipConfig, FlashOpts};
use mcu_bridge::probe::{DebugProbe, WatchKind};
use mcu_bridge::session::Session;
use serde_json::Value;

const CHILD_MODE: &str = "MCU_BRIDGE_JSON_STDIO_CHILD";

struct MockProbe {
    connected: bool,
    halted: bool,
    polls_until_halt: usize,
    polls: usize,
    halt_when_read: bool,
    disconnect_on_poll: bool,
    recovery_fails: bool,
    disconnected: bool,
    calls: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
    detached: Arc<AtomicBool>,
}

impl MockProbe {
    fn record_call(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }
}

impl DebugProbe for MockProbe {
    fn backend_name(&self) -> &'static str {
        "json-stdio-test"
    }

    fn attach(&mut self, _chip: &ChipConfig) -> anyhow::Result<()> {
        self.connected = true;
        Ok(())
    }

    fn detach(&mut self) -> anyhow::Result<()> {
        self.connected = false;
        self.detached.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn try_recover(&mut self) -> anyhow::Result<()> {
        self.record_call();
        if self.recovery_fails {
            anyhow::bail!("mock recovery failed");
        }
        self.connected = true;
        Ok(())
    }

    fn flash(&mut self, _elf: &std::path::Path, _opts: &FlashOpts) -> anyhow::Result<()> {
        self.record_call();
        Ok(())
    }

    fn halt(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        self.record_call();
        self.halted = true;
        Ok(())
    }

    fn resume(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        self.record_call();
        self.halted = false;
        self.polls = 0;
        Ok(())
    }

    fn step(&mut self, _core: Option<usize>) -> anyhow::Result<()> {
        self.record_call();
        self.halted = true;
        Ok(())
    }

    fn core_count(&self) -> usize {
        1
    }

    fn active_core(&self) -> usize {
        0
    }

    fn set_breakpoint(&mut self, _addr: u32, _core: Option<usize>) -> anyhow::Result<usize> {
        self.record_call();
        Ok(1)
    }

    fn clear_breakpoint(&mut self, _id: usize) -> anyhow::Result<()> {
        self.record_call();
        Ok(())
    }

    fn set_watchpoint(&mut self, _addr: u32, _len: u32, _kind: WatchKind) -> anyhow::Result<usize> {
        self.record_call();
        Ok(1)
    }

    fn clear_watchpoint(&mut self, _id: usize) -> anyhow::Result<()> {
        self.record_call();
        Ok(())
    }

    fn read_mem(&mut self, _addr: u32, len: u32, _core: Option<usize>) -> anyhow::Result<Vec<u8>> {
        self.record_call();
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(vec![0; len as usize])
    }

    fn write_mem(&mut self, _addr: u32, _data: &[u8], _core: Option<usize>) -> anyhow::Result<()> {
        self.record_call();
        Ok(())
    }

    fn read_regs(
        &mut self,
        _core: Option<usize>,
    ) -> anyhow::Result<std::collections::HashMap<String, u64>> {
        self.record_call();
        Ok(std::collections::HashMap::from([(
            "pc".into(),
            0x0800_0100,
        )]))
    }

    fn is_halted(&mut self, _core: Option<usize>) -> bool {
        self.halted
    }

    fn poll_halted(&mut self, _core: Option<usize>) -> bool {
        self.record_call();
        self.polls += 1;
        if self.disconnect_on_poll && !self.disconnected {
            self.disconnected = true;
            self.connected = false;
            return false;
        }
        if (self.halt_when_read && self.reads.load(Ordering::SeqCst) > 0)
            || self.polls >= self.polls_until_halt
        {
            self.halted = true;
        }
        self.halted
    }
}

#[test]
#[ignore = "launched as a child process by JSON stdio lifecycle regressions"]
fn json_session_process_fixture() {
    let Some(mode) = std::env::var_os(CHILD_MODE) else {
        return;
    };
    let mode = mode.to_string_lossy().into_owned();
    let detached = Arc::new(AtomicBool::new(false));
    let calls = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    let backend = MockProbe {
        connected: false,
        halted: true,
        polls_until_halt: 2,
        polls: 0,
        halt_when_read: mode == "watch",
        disconnect_on_poll: mode == "disconnect",
        recovery_fails: mode == "disconnect",
        disconnected: false,
        calls: calls.clone(),
        reads: reads.clone(),
        detached: detached.clone(),
    };
    let chip = ChipConfig {
        name: "mock-target".into(),
        architecture: "cortex-m4".into(),
        flash_base: 0x0800_0000,
        flash_size: 1024,
        ram_base: 0x2000_0000,
        ram_size: 1024,
    };
    let session = Session::attach(&chip, Box::new(backend)).unwrap();
    let baseline_calls = calls.load(Ordering::SeqCst);
    let (log_tx, log_rx) = mpsc::channel();
    let log_count = if mode == "backlog" { 40 } else { 1 };
    for index in 0..log_count {
        log_tx
            .send(LogEvent {
                channel: "mock".into(),
                data: format!("log {index}"),
            })
            .unwrap();
    }
    drop(log_tx);
    let mut json_session = JsonSession::new(
        session,
        10,
        16,
        Arc::new(std::sync::RwLock::new(LogBuffer::new(64))),
        Some(log_rx),
        None,
    );

    if mode == "watch" {
        json_session
            .initialize_startup(&["0x20000000:4".into()], true)
            .unwrap();
    }
    let run_result = json_session.run();
    if mode == "broken" {
        assert!(
            run_result.is_err(),
            "broken stdout must fail the JSON session"
        );
        assert!(detached.load(Ordering::SeqCst), "session detach must run");
        std::process::exit(0);
    }
    if mode == "disconnect" {
        assert!(run_result.is_err(), "expected terminal JSON session error");
    } else {
        run_result.unwrap();
    }
    assert!(detached.load(Ordering::SeqCst), "session detach must run");
    if mode == "invalid" {
        assert_eq!(calls.load(Ordering::SeqCst), baseline_calls);
    }
}

#[test]
fn idle_stdin_receives_halt_and_quit_does_not_wait_for_eof() {
    let mut child = spawn_fixture("idle");
    let mut stdin = child.stdin.take().unwrap();
    let (line_rx, reader) = pump_stdout(child.stdout.take().unwrap(), None);

    wait_for_event(&line_rx, "attached", Duration::from_secs(5));
    wait_for_event(&line_rx, "log", Duration::from_secs(5));
    writeln!(stdin, r#"{{"cmd":"resume","id":1}}"#).unwrap();
    wait_for_event(&line_rx, "halted", Duration::from_secs(5));

    writeln!(stdin, r#"{{"cmd":"quit","id":2}}"#).unwrap();
    let status = wait_for_exit(&mut child, Duration::from_secs(5));
    assert!(status.success(), "child test process exited with {status}");
    drop(stdin);
    let _ = reader.join();
}

#[test]
fn watch_startup_samples_before_idle_halt_and_preserves_buffer_history() {
    let mut child = spawn_fixture("watch");
    let mut stdin = child.stdin.take().unwrap();
    let (line_rx, reader) = pump_stdout(child.stdout.take().unwrap(), None);

    wait_for_event(&line_rx, "attached", Duration::from_secs(5));
    wait_for_event(&line_rx, "halted", Duration::from_secs(5));
    writeln!(
        stdin,
        r#"{{"cmd":"buffer","args":{{"since":0,"watch_id":0}},"id":3}}"#
    )
    .unwrap();
    let response = wait_for_response(&line_rx, 3, Duration::from_secs(5));
    assert!(response["data"]["count"].as_u64().unwrap() > 0);

    writeln!(stdin, r#"{{"cmd":"quit","id":4}}"#).unwrap();
    assert!(wait_for_exit(&mut child, Duration::from_secs(5)).success());
    drop(stdin);
    let _ = reader.join();
}

#[test]
fn repeated_resume_halt_cycles_emit_one_event_per_transition() {
    let mut child = spawn_fixture("cycles");
    let mut stdin = child.stdin.take().unwrap();
    let (line_rx, reader) = pump_stdout(child.stdout.take().unwrap(), None);
    wait_for_event(&line_rx, "attached", Duration::from_secs(5));

    for id in [21, 22] {
        writeln!(stdin, r#"{{"cmd":"resume","id":{id}}}"#).unwrap();
        wait_for_response(&line_rx, id, Duration::from_secs(5));
        wait_for_event(&line_rx, "halted", Duration::from_secs(5));
    }
    writeln!(stdin, r#"{{"cmd":"status","id":23}}"#).unwrap();
    let response = wait_for_response(&line_rx, 23, Duration::from_secs(5));
    assert_eq!(response["data"]["state"], "Halted");
    assert_no_event(&line_rx, "halted", Duration::from_millis(100));

    writeln!(stdin, r#"{{"cmd":"quit","id":24}}"#).unwrap();
    assert!(wait_for_exit(&mut child, Duration::from_secs(5)).success());
    drop(stdin);
    let _ = reader.join();
}

#[test]
fn eof_detaches_while_stdin_reader_finishes() {
    let mut child = spawn_fixture("eof");
    let stdin = child.stdin.take().unwrap();
    let (line_rx, reader) = pump_stdout(child.stdout.take().unwrap(), None);
    wait_for_event(&line_rx, "attached", Duration::from_secs(5));
    drop(stdin);
    assert!(wait_for_exit(&mut child, Duration::from_secs(5)).success());
    let _ = reader.join();
}

#[test]
fn broken_pipe_returns_error_and_detaches_session() {
    let mut child = spawn_fixture("broken");
    let mut stdin = child.stdin.take().unwrap();
    let (line_rx, reader) = pump_stdout(child.stdout.take().unwrap(), Some("attached"));
    wait_for_event(&line_rx, "attached", Duration::from_secs(5));
    reader.join().unwrap();
    writeln!(stdin, r#"{{"cmd":"status","id":31}}"#).unwrap();
    let status = wait_for_exit(&mut child, Duration::from_secs(5));
    assert!(
        status.success(),
        "child fixture failed after broken pipe: {status}"
    );
    drop(stdin);
}

#[test]
fn failed_probe_recovery_is_reported_before_session_exit() {
    let mut child = spawn_fixture("disconnect");
    let mut stdin = child.stdin.take().unwrap();
    let (line_rx, reader) = pump_stdout(child.stdout.take().unwrap(), None);
    wait_for_event(&line_rx, "attached", Duration::from_secs(5));
    writeln!(stdin, r#"{{"cmd":"resume","id":41}}"#).unwrap();
    wait_for_response(&line_rx, 41, Duration::from_secs(5));
    let recovering = wait_for_event_value(&line_rx, "probe_recovering", Duration::from_secs(5));
    assert_eq!(recovering["data"]["code"], "E_PROBE");
    let lost = wait_for_event_value(&line_rx, "probe_lost", Duration::from_secs(5));
    assert_eq!(lost["data"]["code"], "E_PROBE_LOST");
    assert!(wait_for_exit(&mut child, Duration::from_secs(5)).success());
    drop(stdin);
    let _ = reader.join();
}

#[test]
fn requests_are_served_while_log_events_are_backlogged() {
    let mut child = spawn_fixture("backlog");
    let mut stdin = child.stdin.take().unwrap();
    let (line_rx, reader) = pump_stdout(child.stdout.take().unwrap(), None);
    wait_for_event(&line_rx, "attached", Duration::from_secs(5));
    writeln!(stdin, r#"{{"cmd":"status","id":51}}"#).unwrap();
    writeln!(stdin, r#"{{"cmd":"status","id":52}}"#).unwrap();
    wait_for_response(&line_rx, 51, Duration::from_secs(5));
    wait_for_response(&line_rx, 52, Duration::from_secs(5));
    writeln!(stdin, r#"{{"cmd":"quit","id":53}}"#).unwrap();
    assert!(wait_for_exit(&mut child, Duration::from_secs(5)).success());
    drop(stdin);
    let _ = reader.join();
}

#[test]
fn invalid_numeric_requests_do_not_call_backend() {
    let mut child = spawn_fixture("invalid");
    let mut stdin = child.stdin.take().unwrap();
    let (line_rx, reader) = pump_stdout(child.stdout.take().unwrap(), None);
    wait_for_event(&line_rx, "attached", Duration::from_secs(5));
    writeln!(
        stdin,
        r#"{{"cmd":"mem","args":{{"addr":4294967296,"len":4}},"id":61}}"#
    )
    .unwrap();
    writeln!(
        stdin,
        r#"{{"cmd":"watch","args":{{"addr":4,"size":4294967296}},"id":62}}"#
    )
    .unwrap();
    assert_eq!(
        wait_for_response(&line_rx, 61, Duration::from_secs(5))["error"]["code"],
        "E_PARAM"
    );
    assert_eq!(
        wait_for_response(&line_rx, 62, Duration::from_secs(5))["error"]["code"],
        "E_PARAM"
    );
    writeln!(stdin, r#"{{"cmd":"quit","id":63}}"#).unwrap();
    assert!(wait_for_exit(&mut child, Duration::from_secs(5)).success());
    drop(stdin);
    let _ = reader.join();
}

fn spawn_fixture(mode: &str) -> ChildGuard {
    ChildGuard(
        ProcessCommand::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "json_session_process_fixture",
                "--ignored",
                "--nocapture",
            ])
            .env(CHILD_MODE, mode)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap(),
    )
}

struct ChildGuard(Child);

impl Deref for ChildGuard {
    type Target = Child;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChildGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn pump_stdout(
    stdout: std::process::ChildStdout,
    close_after_event: Option<&'static str>,
) -> (Receiver<String>, thread::JoinHandle<()>) {
    let (tx, rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        let mut lines = BufReader::new(stdout).lines();
        while let Some(Ok(line)) = lines.next() {
            let close = close_after_event.is_some_and(|event| {
                serde_json::from_str::<Value>(&line)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("event")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .as_deref()
                    == Some(event)
            });
            if tx.send(line).is_err() || close {
                break;
            }
        }
    });
    (rx, reader)
}

fn wait_for_event(rx: &Receiver<String>, expected: &str, timeout: Duration) {
    let _ = wait_for_event_value(rx, expected, timeout);
}

fn wait_for_event_value(rx: &Receiver<String>, expected: &str, timeout: Duration) -> Value {
    wait_for_value(rx, timeout, |value| {
        value.get("event").and_then(Value::as_str) == Some(expected)
    })
}

fn wait_for_response(rx: &Receiver<String>, id: u64, timeout: Duration) -> Value {
    wait_for_value(rx, timeout, |value| {
        value.get("id").and_then(Value::as_u64) == Some(id)
    })
}

fn wait_for_value(
    rx: &Receiver<String>,
    timeout: Duration,
    matches: impl Fn(&Value) -> bool,
) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let line = rx
            .recv_timeout(remaining)
            .unwrap_or_else(|error| panic!("timed out waiting for JSON output: {error}"));
        if let Ok(value) = serde_json::from_str::<Value>(&line)
            && matches(&value)
        {
            return value;
        }
    }
}

fn assert_no_event(rx: &Receiver<String>, forbidden: &str, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(remaining) {
            Ok(line) => {
                if let Ok(value) = serde_json::from_str::<Value>(&line) {
                    assert_ne!(value.get("event").and_then(Value::as_str), Some(forbidden));
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => return,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> std::process::ExitStatus {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child process did not exit before timeout");
        }
        thread::sleep(Duration::from_millis(10));
    }
}
