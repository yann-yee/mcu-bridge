//! SerialMonitor — 日志通道接收线程（占位，后续实现）。
//!
//! 设计文档 §3.3：独立线程持有 `Box<dyn LogChannel>` 持续读取，
//! 日志数据写入 ring buffer 供 Agent 通过 `serial read` 命令查阅。

/// 日志通道接收监控器。
pub struct SerialMonitor;
