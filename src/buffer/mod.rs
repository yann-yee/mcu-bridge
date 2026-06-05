//! 调试缓冲层 — 核心差异化能力。
//!
//! 设计文档 §3.2 定义了 ring buffer + 定时采样机制。
//! Agent 不与实时 MCU 交互，与缓冲的历史数据交互。
//!
//! ⚠ P2/P3 预留项标记了暂时未从调用链触达的公共 API 元素。

#![allow(dead_code)]
pub mod serial;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use serde::Serialize;

use crate::probe::DebugProbe;
use crate::probe::WatchKind;

/// 单次采样记录 — ring buffer 中的一条数据。
///
/// 设计文档 buffer schema:
///   sn, tick_us, val, core, bp_flag, gap, regs, old_val, new_val
#[derive(Debug, Clone, Serialize)]
pub struct Sample {
    /// 全局递增序列号，Agent 增量查询 (buffer --since N)
    pub sn: u64,
    /// μs 时间戳
    pub tick_us: u64,
    /// 采样值
    pub val: u64,
    /// 来源核心号
    pub core: usize,
    /// 断点命中标记（此时 regs 有值）
    pub bp_flag: bool,
    /// 连接恢复标记（探针断连期间数据丢失）
    pub gap: bool,
    /// 寄存器快照（仅 bp_flag = true 时有值）
    pub regs: Option<HashMap<String, u64>>,
    /// watchpoint 触发时的旧值
    pub old_val: Option<u64>,
    /// watchpoint 触发时的新值
    pub new_val: Option<u64>,
}

/// 日志条目 — 日志 ring buffer 中的一条数据。
///
/// SerialMonitor 线程写入，Agent/CLI 通过 serial 命令读取。
#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    /// 全局递增序列号
    pub sn: u64,
    /// μs 时间戳（主机侧）
    pub tick_us: u64,
    /// 来源通道: "rtt" | "uart" | "semihosting"
    pub channel: String,
    /// 日志文本
    pub data: String,
}

/// 日志环形缓冲区。
///
/// 固定容量，写满后覆盖最旧记录。
/// 与 DebugBuffer 的 ring buffer 语义一致。
#[derive(Debug, Clone)]
pub struct LogBuffer {
    /// 环形缓冲
    pub entries: Vec<LogEntry>,
    /// 最大容量
    pub capacity: usize,
    /// 全局递增序列号
    pub global_sn: u64,
}

impl LogBuffer {
    /// 创建一个指定容量的日志缓冲区。
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: Vec::with_capacity(capacity),
            capacity,
            global_sn: 0,
        }
    }

    /// 追加一条日志条目（写满后覆盖最旧记录）。
    /// 空 channel 名称会回退为 "?"。
    pub fn push(&mut self, channel: &str, data: String) {
        self.global_sn += 1;
        let now_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64;
        let channel_name = if channel.is_empty() { "?" } else { channel };
        let entry = LogEntry {
            sn: self.global_sn,
            tick_us: now_us,
            channel: channel_name.to_string(),
            data,
        };
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push(entry);
    }

    /// 查询 `sn >= since` 的日志条目。
    pub fn get_since(&self, since: u64) -> Vec<&LogEntry> {
        self.entries.iter().filter(|e| e.sn >= since).collect()
    }

    /// 返回当前条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 缓冲区是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// 一个数据观测目标。
///
/// 用户通过 `watch <variable> <size>` 命令设定。
/// 每个 target 有独立的 ring buffer。
#[derive(Debug, Clone)]
pub struct WatchTarget {
    /// 用户侧 ID
    pub id: usize,
    /// 标签（变量名或地址）
    pub label: String,
    /// 内存地址
    pub addr: u32,
    /// 观测大小 (bytes): 1|2|4|8
    pub size: u32,
    /// 触发类型
    pub kind: WatchKind,
}

/// 调试数据缓冲区。
///
/// 每个 watch target 一个独立的 Vec<Sample> ring buffer。
/// 定时采样线程写入，CLI/Agent 线程读取。
pub struct DebugBuffer {
    /// 所有观测目标
    pub targets: Vec<WatchTarget>,
    /// 每个 target 的采样历史 (key = watch id)
    pub samples: HashMap<usize, Vec<Sample>>,
    /// 每个 target ring buffer 的最大容量
    pub capacity: usize,
    /// 全局递增序列号
    pub global_sn: u64,
}

impl DebugBuffer {
    /// 创建一个指定容量的缓冲区。
    pub fn new(capacity: usize) -> Self {
        Self {
            targets: Vec::new(),
            samples: HashMap::new(),
            capacity,
            global_sn: 0,
        }
    }

    /// 向指定 watch target 的 ring buffer 追加一条采样。
    ///
    /// 写满后覆盖最旧记录（ring buffer 语义）。
    pub fn push_sample(&mut self, watch_id: usize, mut sample: Sample) {
        self.global_sn += 1;
        sample.sn = self.global_sn;

        let buf = self
            .samples
            .entry(watch_id)
            .or_insert_with(|| Vec::with_capacity(self.capacity));

        if buf.len() >= self.capacity {
            // 写满：移除最旧记录
            buf.remove(0);
        }
        buf.push(sample);
    }

    /// 解析 `--watch` 参数格式 `addr:size[:label]`。
    ///
    /// 返回 (addr, size, Option<label>)。
    /// 如果标签未指定，则自动生成 `0x{addr:08x}` 作为标签。
    pub fn parse_watch_spec(input: &str) -> Result<(u32, u32, Option<String>), String> {
        let parts: Vec<&str> = input.split(':').collect();
        if parts.len() < 2 {
            return Err(
                "invalid watch syntax. Use addr:size[:label], e.g. 0x20000000:4:counter".into(),
            );
        }
        let addr = parse_u32(parts[0]).map_err(|_| {
            format!(
                "invalid address: '{}'. Use hex (0x...) or decimal.",
                parts[0]
            )
        })?;
        let size = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("invalid size: '{}'. Use decimal (1, 2, 4, or 8).", parts[1]))?;
        if !matches!(size, 1 | 2 | 4 | 8) {
            return Err(format!("watch size must be 1, 2, 4, or 8, got {size}"));
        }
        let label = parts
            .get(2)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        Ok((addr, size, label))
    }

    /// 添加一个 watch target，返回 target ID。
    pub fn add_target(&mut self, addr: u32, size: u32, label: Option<String>) -> usize {
        let id = self.targets.len();
        let label = label.unwrap_or_else(|| format!("0x{addr:08x}"));
        self.targets.push(WatchTarget {
            id,
            label,
            addr,
            size,
            kind: WatchKind::Read,
        });
        self.samples
            .entry(id)
            .or_insert_with(|| Vec::with_capacity(self.capacity));
        id
    }

    /// 查询采样历史。
    ///
    /// * `watch_id` — 如果为 None，返回所有 target 的历史
    /// * `since` — 如果为 Some，只返回 sn >= since 的记录
    pub fn get_samples(&self, watch_id: Option<usize>, since: Option<u64>) -> Vec<Vec<Sample>> {
        let active_ids: Vec<usize> = match watch_id {
            Some(id) => vec![id],
            None => self.targets.iter().map(|t| t.id).collect(),
        };
        active_ids
            .into_iter()
            .filter_map(|id| self.samples.get(&id))
            .map(|buf| match since {
                Some(sn) => buf.iter().filter(|s| s.sn >= sn).cloned().collect(),
                None => buf.clone(),
            })
            .collect()
    }

    /// 获取所有 target 的最新状态摘要（用于 buffer 命令显示）。
    pub fn summarize(&self) -> Vec<HashMap<&str, serde_json::Value>> {
        self.targets
            .iter()
            .map(|t| {
                let mut map = HashMap::new();
                map.insert("id", serde_json::Value::Number(t.id.into()));
                map.insert("label", serde_json::Value::String(t.label.clone()));
                map.insert("addr", serde_json::Value::Number(t.addr.into()));
                map.insert("size", serde_json::Value::Number(t.size.into()));
                // 最新值
                let latest = self.samples.get(&t.id).and_then(|buf| buf.last());
                if let Some(sample) = latest {
                    map.insert("sn", serde_json::Value::Number(sample.sn.into()));
                    map.insert("val", serde_json::Value::Number(sample.val.into()));
                    map.insert("tick_us", serde_json::Value::Number(sample.tick_us.into()));
                }
                map
            })
            .collect()
    }
}

/// 解析十六进制或十进制地址字符串为 u32。
fn parse_u32(s: &str) -> Result<u32, std::num::ParseIntError> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16)
    } else {
        s.parse::<u32>()
    }
}

/// 定时采样器。
///
/// 在独立线程中运行，定时读取所有 watch target 的内存值，
/// 写入 DebugBuffer，同时检测断点命中。
pub struct Sampler {
    /// 共享后端引用
    backend: Arc<Mutex<Box<dyn DebugProbe>>>,
    /// 共享缓冲区引用
    buffer: Arc<RwLock<DebugBuffer>>,
    /// 采样间隔
    interval: Duration,
    /// 停止信号
    stop_flag: Arc<AtomicBool>,
    /// 活跃核编号
    active_core: usize,
    /// 剩余重试次数 (max 3)
    retries_left: AtomicU32,
}

impl Sampler {
    /// 创建采样器。
    pub fn new(
        backend: Arc<Mutex<Box<dyn DebugProbe>>>,
        buffer: Arc<RwLock<DebugBuffer>>,
        interval_ms: u64,
        active_core: usize,
    ) -> Self {
        Self {
            backend,
            buffer,
            interval: Duration::from_millis(interval_ms),
            stop_flag: Arc::new(AtomicBool::new(false)),
            active_core,
            retries_left: AtomicU32::new(3),
        }
    }

    /// 获取停止信号（主线程用于发送停止）。
    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        self.stop_flag.clone()
    }

    /// 采样主循环 — 在独立线程中运行。
    ///
    /// 每 `interval` 执行一轮采样：
    /// 1. 检查 stop_flag
    /// 2. sleep(interval)
    /// 3. 锁定 backend
    /// 4. 遍历所有 watch target 读取内存
    /// 5. 写入 ring buffer
    /// 6. `poll_halted()` 检测断点
    /// 7. read_mem 失败时自动触发探针自恢复
    pub fn run(&mut self) {
        'outer: loop {
            if self.stop_flag.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(self.interval);

            // 锁 backend
            let mut guard = match self.backend.lock() {
                Ok(g) => g,
                Err(_) => continue, // 锁被毒化，跳过本轮
            };

            // 读取所有 watch target
            let targets: Vec<(usize, u32, u32)> = match self.buffer.read() {
                Ok(buf) => buf.targets.iter().map(|t| (t.id, t.addr, t.size)).collect(),
                Err(_) => continue,
            };

            let now_us = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_micros() as u64;

            for (id, addr, size) in &targets {
                let data = guard.read_mem(*addr, *size, Some(self.active_core));
                match data {
                    Ok(bytes) => {
                        let val = bytes_to_u64(&bytes);
                        let sample = Sample {
                            sn: 0, // 由 push_sample 赋值
                            tick_us: now_us,
                            val,
                            core: self.active_core,
                            bp_flag: false,
                            gap: false,
                            regs: None,
                            old_val: None,
                            new_val: None,
                        };
                        if let Ok(mut buf) = self.buffer.write() {
                            buf.push_sample(*id, sample);
                        }
                        // read_mem 成功 → 重置重试计数
                        self.retries_left.store(3, Ordering::Relaxed);
                    }
                    Err(e) => {
                        log::warn!("sampler: read_mem at 0x{addr:08x} failed: {e}");
                        // 释放 backend 锁，尝试 recovery
                        drop(guard);
                        let recovered = self.attempt_recovery();
                        if recovered {
                            // 重新锁 backend 继续
                            guard = match self.backend.lock() {
                                Ok(g) => g,
                                Err(_) => {
                                    self.stop_flag.store(true, Ordering::Relaxed);
                                    break 'outer;
                                }
                            };
                            // 记录 gap=true 采样
                            let gap_sample = Sample {
                                sn: 0,
                                tick_us: now_us,
                                val: 0,
                                core: self.active_core,
                                bp_flag: false,
                                gap: true,
                                regs: None,
                                old_val: None,
                                new_val: None,
                            };
                            if let Ok(mut buf) = self.buffer.write() {
                                buf.push_sample(*id, gap_sample);
                            }
                            self.retries_left.store(3, Ordering::Relaxed);
                        } else {
                            log::error!("sampler: recovery failed after 3 attempts, stopping");
                            self.stop_flag.store(true, Ordering::Relaxed);
                            break 'outer;
                        }
                    }
                }
            }

            // 断点检测（检查 stop_flag 防止已标记退出后仍操作 guard）
            if self.stop_flag.load(Ordering::Relaxed) {
                break;
            }
            if guard.poll_halted(Some(self.active_core)) {
                self.stop_flag.store(true, Ordering::Relaxed);
                break;
            }
        }
    }

    /// 尝试恢复探针连接，最多 3 次，间隔 500ms。
    /// 返回 true 表示恢复成功。
    fn attempt_recovery(&mut self) -> bool {
        let max_retries = self.retries_left.load(Ordering::Relaxed);
        for attempt in 1..=max_retries {
            std::thread::sleep(Duration::from_millis(500));
            log::info!("sampler: recovery attempt {}/{}", attempt, max_retries);
            let mut guard = match self.backend.lock() {
                Ok(g) => g,
                Err(_) => continue,
            };
            match guard.try_recover() {
                Ok(()) => {
                    log::info!("sampler: recovery successful on attempt {}", attempt);
                    return true;
                }
                Err(e) => {
                    log::warn!(
                        "sampler: recovery attempt {}/{} failed: {e}",
                        attempt,
                        max_retries
                    );
                }
            }
        }
        false
    }
}

/// 将字节数组转换为 u64（小端序）。
fn bytes_to_u64(bytes: &[u8]) -> u64 {
    match bytes.len() {
        0 => 0,
        1 => bytes[0] as u64,
        2 => u16::from_le_bytes([bytes[0], bytes[1]]) as u64,
        4 => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as u64,
        _ => {
            let mut arr = [0u8; 8];
            let n = bytes.len().min(8);
            arr[..n].copy_from_slice(&bytes[..n]);
            u64::from_le_bytes(arr)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_watch_addr_size() {
        let (addr, size, label) = DebugBuffer::parse_watch_spec("0x20000000:4").unwrap();
        assert_eq!(addr, 0x20000000);
        assert_eq!(size, 4);
        assert!(label.is_none());
    }

    #[test]
    fn test_parse_watch_addr_size_label() {
        let (addr, size, label) = DebugBuffer::parse_watch_spec("0x20000000:4:counter").unwrap();
        assert_eq!(addr, 0x20000000);
        assert_eq!(size, 4);
        assert_eq!(label.unwrap(), "counter");
    }

    #[test]
    fn test_parse_watch_invalid() {
        assert!(DebugBuffer::parse_watch_spec("invalid").is_err());
    }

    #[test]
    fn test_parse_watch_size_too_large() {
        assert!(DebugBuffer::parse_watch_spec("0x20000000:16").is_err());
    }

    #[test]
    fn test_push_sample_ring_behavior() {
        let mut buf = DebugBuffer::new(3); // capacity = 3
        buf.add_target(0x20000000, 4, Some("test".into()));
        for _ in 0..5 {
            buf.push_sample(
                0,
                Sample {
                    sn: 0,
                    tick_us: 0,
                    val: 42,
                    core: 0,
                    bp_flag: false,
                    gap: false,
                    regs: None,
                    old_val: None,
                    new_val: None,
                },
            );
        }
        // capacity 3, pushed 5 -> should keep only last 3 (sn=3,4,5)
        let samples = buf.get_samples(Some(0), None);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].len(), 3);
        assert_eq!(samples[0][0].sn, 3);
        assert_eq!(samples[0][1].sn, 4);
        assert_eq!(samples[0][2].sn, 5);
    }

    #[test]
    fn test_get_samples_since() {
        let mut buf = DebugBuffer::new(10);
        buf.add_target(0x20000000, 4, None);
        for _ in 0..5 {
            buf.push_sample(
                0,
                Sample {
                    sn: 0,
                    tick_us: 0,
                    val: 42,
                    core: 0,
                    bp_flag: false,
                    gap: false,
                    regs: None,
                    old_val: None,
                    new_val: None,
                },
            );
        }
        let samples = buf.get_samples(Some(0), Some(3));
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].len(), 3);
        assert_eq!(samples[0][0].sn, 3);
        assert_eq!(samples[0][1].sn, 4);
        assert_eq!(samples[0][2].sn, 5);
    }

    #[test]
    fn test_add_target() {
        let mut buf = DebugBuffer::new(128);
        let id = buf.add_target(0x20000000, 4, Some("counter".into()));
        assert_eq!(id, 0);
        assert_eq!(buf.targets.len(), 1);
        assert_eq!(buf.targets[0].label, "counter");
    }

    #[test]
    fn test_add_target_auto_label() {
        let mut buf = DebugBuffer::new(128);
        let id = buf.add_target(0x20000000, 4, None);
        assert_eq!(id, 0);
        assert_eq!(buf.targets[0].label, "0x20000000");
    }

    #[test]
    fn test_bytes_to_u64() {
        assert_eq!(bytes_to_u64(&[0x01]), 1);
        assert_eq!(bytes_to_u64(&[0x78, 0x56, 0x34, 0x12]), 0x12345678);
        assert_eq!(
            bytes_to_u64(&[0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01]),
            0x0123456789abcdef
        );
    }

    #[test]
    fn test_sampler_stop_flag() {
        let probe: Box<dyn DebugProbe> = Box::new(crate::probe::probe_rs::ProbeRsBackend::new());
        let buf = Arc::new(RwLock::new(DebugBuffer::new(128)));
        let backend = Arc::new(Mutex::new(probe));
        let mut sampler = Sampler::new(backend, buf, 10, 0);
        sampler.stop_flag().store(true, Ordering::Relaxed);
        sampler.run();
        // 没有 panic 即通过
    }

    #[test]
    fn test_empty_buffer_queries() {
        let buf = DebugBuffer::new(128);
        let samples = buf.get_samples(None, None);
        assert!(samples.is_empty() || samples.iter().all(|v| v.is_empty()));
    }

    // ── LogBuffer 测试 ──

    #[test]
    fn test_log_buffer_push_and_len() {
        let mut buf = LogBuffer::new(10);
        assert!(buf.is_empty());
        buf.push("rtt", "hello".into());
        assert_eq!(buf.len(), 1);
        buf.push("rtt", "world".into());
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_log_buffer_ring_overflow() {
        let mut buf = LogBuffer::new(3);
        for i in 0..5 {
            buf.push("rtt", format!("msg {i}"));
        }
        // capacity 3, pushed 5 → keep last 3 (sn=3,4,5)
        assert_eq!(buf.len(), 3);
        let entries = buf.get_since(0);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].sn, 3);
        assert_eq!(entries[1].sn, 4);
        assert_eq!(entries[2].sn, 5);
    }

    #[test]
    fn test_log_buffer_get_since() {
        let mut buf = LogBuffer::new(10);
        for i in 0..5 {
            buf.push("uart", format!("log {i}"));
        }
        let entries = buf.get_since(3);
        assert_eq!(entries.len(), 3); // sn 3,4,5
        assert_eq!(entries[0].sn, 3);
        assert_eq!(entries[2].sn, 5);
    }

    #[test]
    fn test_log_buffer_channel_name() {
        let mut buf = LogBuffer::new(10);
        buf.push("rtt", "a".into());
        buf.push("uart", "b".into());
        buf.push("semihosting", "c".into());
        let entries = buf.get_since(1);
        assert_eq!(entries[0].channel, "rtt");
        assert_eq!(entries[1].channel, "uart");
        assert_eq!(entries[2].channel, "semihosting");
    }

    #[test]
    fn test_log_buffer_empty_channel_fallback() {
        let mut buf = LogBuffer::new(10);
        buf.push("", "data with empty channel".into());
        let entries = buf.get_since(1);
        assert_eq!(entries.len(), 1);
        // 空 channel 应回退为 "?"
        assert_eq!(entries[0].channel, "?");
    }
}
