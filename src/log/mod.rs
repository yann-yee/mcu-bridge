/// 日志通道抽象层 — RTT / UART / Semihosting 统一为 MCU↔主机文本字节流。
///
/// 设计文档 §3.3 定义了 6 个方法的 trait。
/// `SerialMonitor` 线程持有 `Box<dyn LogChannel>` 持续读取日志数据。
pub mod rtt;
pub mod semihosting;
pub mod uart;

/// 日志通道 trait。
///
/// 所有实现必须同时实现 `Send`。
pub trait LogChannel: Send {
    /// 通道名称: "rtt" | "uart" | "semihosting"
    fn name(&self) -> &str;

    /// 打开通道（建立连接）
    fn open(&mut self) -> anyhow::Result<()>;

    /// 从通道读取数据，返回读取的字节数
    fn read(&mut self, buf: &mut [u8]) -> anyhow::Result<usize>;

    /// 向通道写入数据
    fn write(&mut self, data: &[u8]) -> anyhow::Result<()>;

    /// 通道是否可写（Semihosting 只读不可写）
    fn is_writable(&self) -> bool;

    /// 关闭通道
    fn close(&mut self) -> anyhow::Result<()>;
}
