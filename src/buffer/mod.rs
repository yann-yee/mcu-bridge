/// 调试缓冲层 — 核心差异化能力。
///
/// 设计文档 §3.2 定义了 ring buffer + 定时采样机制。
/// Agent 不与实时 MCU 交互，与缓冲的历史数据交互。
pub mod serial;

use crate::probe::WatchKind;
use std::collections::HashMap;

/// 单次采样记录 — ring buffer 中的一条数据。
///
/// 设计文档 buffer schema:
///   sn, tick_us, val, core, bp_flag, gap, regs, old_val, new_val
#[derive(Debug, Clone)]
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
#[derive(Debug)]
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
}
