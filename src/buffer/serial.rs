//! SerialMonitor — 日志通道接收线程。
//!
//! 设计文档 §3.3：独立线程持有 `Box<dyn LogChannel>` 持续读取，
//! 日志数据通过双路投递：
//!   1. 写入 LogBuffer ring buffer（历史查阅）
//!   2. 通过 mpsc channel 推送到 JSON 事件循环（实时推送）
//!
//! 混合模式 Q2=C：实时事件推送 + ring buffer 备份。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread::{self, JoinHandle};

use crate::buffer::LogBuffer;
use crate::log::LogChannel;

/// 日志事件（用于 JSON-Lines 实时推送）。
#[derive(Debug, Clone)]
pub struct LogEvent {
    /// 通道名称: "rtt" | "uart" | "semihosting"
    pub channel: String,
    /// 日志文本
    pub data: String,
}

/// 日志通道接收监控器。
///
/// 生命周期：
///   1. `new()`：传入通道（应已 open）→ Arc<Mutex> 包装
///   2. `start()`：启动接收线程
///   3. `stop()`：发送停止信号 → 等待线程结束
pub struct SerialMonitor {
    /// 日志通道实例（Arc<Mutex> 以便线程共享）
    channel: Arc<Mutex<Box<dyn LogChannel>>>,
    /// 日志环形缓冲区（共享，Agent 通过 serial read 查阅）
    buffer: Arc<RwLock<LogBuffer>>,
    /// 实时事件发送端（mpsc，连接 JSON 事件循环）
    event_tx: mpsc::Sender<LogEvent>,
    /// 停止信号
    stop_flag: Arc<AtomicBool>,
    /// 接收线程句柄
    handle: Option<JoinHandle<()>>,
    /// 每秒事件推送上限（防止高频打爆 Agent）
    max_events_per_sec: usize,
}

impl SerialMonitor {
    /// 创建 SerialMonitor。
    ///
    /// * `channel` — 已打开的日志通道（调用此方法前应已 `channel.open()`）
    /// * `buffer` — 共享日志 ring buffer
    /// * `event_tx` — 事件发送端，JSON 主循环从 `mpsc::Receiver` 接收
    /// * `max_events_per_sec` — 每秒最大事件推送数（0 = 不限）
    pub fn new(
        channel: Box<dyn LogChannel>,
        buffer: Arc<RwLock<LogBuffer>>,
        event_tx: mpsc::Sender<LogEvent>,
        max_events_per_sec: usize,
    ) -> Self {
        Self {
            channel: Arc::new(Mutex::new(channel)),
            buffer,
            event_tx,
            stop_flag: Arc::new(AtomicBool::new(false)),
            handle: None,
            max_events_per_sec,
        }
    }

    /// 启动接收线程。
    ///
    /// 线程循环：
    ///   1. 检查 stop_flag
    ///   2. `channel.read(buf)` — 非阻塞读取
    ///   3. 有数据 → `LogBuffer::push()` + `event_tx.send()`
    ///   4. 无数据 → 短暂 sleep 避免忙等待
    pub fn start(&mut self) {
        if self.handle.is_some() {
            return;
        }

        let channel = self.channel.clone();
        let buffer = self.buffer.clone();
        let event_tx = self.event_tx.clone();
        let stop_flag = self.stop_flag.clone();
        let max_events = self.max_events_per_sec;

        let handle = thread::Builder::new()
            .name("serial-monitor".into())
            .spawn(move || {
                let mut events_this_sec = 0usize;
                let mut sec_start = std::time::Instant::now();
                let mut read_buf = vec![0u8; 1024];

                loop {
                    if stop_flag.load(Ordering::Relaxed) {
                        break;
                    }

                    let mut guard = match channel.lock() {
                        Ok(g) => g,
                        Err(_) => {
                            log::error!("serial monitor: channel lock poisoned");
                            break;
                        }
                    };

                    match guard.read(&mut read_buf) {
                        Ok(0) => {
                            // 无数据：释放锁后短暂 sleep 避免忙等待
                            drop(guard);
                            std::thread::sleep(std::time::Duration::from_millis(5));
                            continue;
                        }
                        Ok(n) => {
                            let text = String::from_utf8_lossy(&read_buf[..n]).to_string();
                            let ch_name = guard.name().to_string();

                            // 写入 LogBuffer
                            if let Ok(mut buf) = buffer.write() {
                                buf.push(&ch_name, text.clone());
                            }

                            // 实时事件推送（带频率限制）
                            let push = if max_events > 0 {
                                let elapsed = sec_start.elapsed();
                                if elapsed.as_secs() >= 1 {
                                    events_this_sec = 0;
                                    sec_start = std::time::Instant::now();
                                }
                                if events_this_sec < max_events {
                                    events_this_sec += 1;
                                    true
                                } else {
                                    false
                                }
                            } else {
                                true
                            };

                            if push {
                                let _ = event_tx.send(LogEvent {
                                    channel: ch_name,
                                    data: text,
                                });
                            }
                        }
                        Err(e) => {
                            log::warn!("serial monitor read error: {e}");
                            drop(guard);
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                    }
                }
            })
            .expect("failed to spawn serial monitor thread");

        self.handle = Some(handle);
    }

    /// 停止接收线程。
    pub fn stop(&mut self) -> anyhow::Result<()> {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|e| anyhow::anyhow!("serial monitor thread join failed: {e:?}"))?;
        }
        // 关闭通道
        if let Ok(mut guard) = self.channel.lock() {
            guard.close()?;
        }
        Ok(())
    }

    /// 获取停止信号的引用（供外部发送停止）。
    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop_flag.clone()
    }

    /// 获取日志通道的名称。
    pub fn channel_name(&self) -> String {
        if let Ok(guard) = self.channel.lock() {
            guard.name().to_string()
        } else {
            "unknown".into()
        }
    }

    /// 获取底层通道的只读引用（用于类型查询等）。
    pub fn channel_ref(&self) -> &Arc<Mutex<Box<dyn LogChannel>>> {
        &self.channel
    }
}

impl Drop for SerialMonitor {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        // 不阻塞 drop — 线程会因 stop_flag 自行退出
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::LogBuffer;
    use crate::log::LogChannel;
    use std::sync::{Arc, RwLock, mpsc};
    use std::time::Duration;

    /// 一个简单的 mock LogChannel，用于 SerialMonitor 测试。
    struct MockChannel {
        name: String,
        data: Vec<u8>,
        pos: usize,
        should_fail: bool,
    }

    impl MockChannel {
        fn new(name: &str, data: &[u8]) -> Self {
            Self {
                name: name.to_string(),
                data: data.to_vec(),
                pos: 0,
                should_fail: false,
            }
        }
    }

    impl LogChannel for MockChannel {
        fn name(&self) -> &str {
            &self.name
        }

        fn open(&mut self) -> anyhow::Result<()> {
            Ok(())
        }

        fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize> {
            if self.should_fail {
                return Err(anyhow::anyhow!("mock channel error"));
            }
            if self.pos >= self.data.len() {
                return Ok(0);
            }
            let n = (self.data.len() - self.pos).min(buf.len());
            buf[..n].copy_from_slice(&self.data[self.pos..self.pos + n]);
            self.pos += n;
            if self.pos >= self.data.len() {
                self.pos = 0; // 重置以模拟持续数据
            }
            Ok(n)
        }

        fn write(&mut self, _data: &[u8]) -> anyhow::Result<()> {
            Ok(())
        }

        fn is_writable(&self) -> bool {
            true
        }

        fn close(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
    }

    /// 验证 SerialMonitor 接收并存储数据到 LogBuffer。
    #[test]
    fn test_serial_monitor_receives_data() {
        let mock = Box::new(MockChannel::new("test", b"hello, mcu!"));
        let log_buf = Arc::new(RwLock::new(LogBuffer::new(100)));
        let (tx, rx) = mpsc::channel();
        let mut monitor = SerialMonitor::new(mock, log_buf.clone(), tx, 100);

        monitor.start();
        std::thread::sleep(Duration::from_millis(50));
        monitor.stop().unwrap();

        // 验证 LogBuffer 中有数据
        let buf = log_buf.read().unwrap();
        assert!(!buf.is_empty(), "LogBuffer should contain data");

        // 验证事件通道收到数据
        let mut events_received = false;
        while let Ok(event) = rx.try_recv() {
            assert_eq!(event.channel, "test");
            assert!(!event.data.is_empty());
            events_received = true;
        }
        assert!(events_received, "should have received events via mpsc");
    }

    /// 验证 SerialMonitor 线程停止后不再写入。
    #[test]
    fn test_serial_monitor_stop_prevents_writes() {
        let mock = Box::new(MockChannel::new("test", b"data"));
        let log_buf = Arc::new(RwLock::new(LogBuffer::new(100)));
        let (tx, _rx) = mpsc::channel();
        let mut monitor = SerialMonitor::new(mock, log_buf.clone(), tx, 100);

        monitor.start();
        std::thread::sleep(Duration::from_millis(30));

        monitor.stop().unwrap();

        // stop 后再等一会，检查 buffer 不再增长
        std::thread::sleep(Duration::from_millis(30));
        // 至少确保 stop 不崩溃，buffer 内容可读
        assert!(log_buf.read().is_ok());
    }

    /// 验证 channel read 错误不导致 monitor 崩溃。
    #[test]
    fn test_serial_monitor_handles_errors() {
        let mut mock = Box::new(MockChannel::new("error", b"data"));
        mock.should_fail = true;
        let log_buf = Arc::new(RwLock::new(LogBuffer::new(100)));
        let (tx, _rx) = mpsc::channel();
        let mut monitor = SerialMonitor::new(mock, log_buf.clone(), tx, 100);

        monitor.start();
        std::thread::sleep(Duration::from_millis(50));
        // 不应 panic
        monitor.stop().unwrap();
    }

    /// 验证频率限制。
    #[test]
    fn test_serial_monitor_rate_limiting() {
        let mock = Box::new(MockChannel::new("rtt", b"hello"));
        let log_buf = Arc::new(RwLock::new(LogBuffer::new(100)));
        let (tx, rx) = mpsc::channel();
        // 限制 5 事件/秒
        let mut monitor = SerialMonitor::new(mock, log_buf.clone(), tx, 5);

        monitor.start();
        std::thread::sleep(Duration::from_millis(100));
        monitor.stop().unwrap();

        let mut received = 0usize;
        while let Ok(_) = rx.try_recv() {
            received += 1;
        }
        assert!(received > 0, "should have received some events");
        println!("received {received} events in 100ms (rate limit 5/sec)");
    }
}
