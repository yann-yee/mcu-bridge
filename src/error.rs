/// 统一错误类型 — 映射到 JSON-Lines 协议的 12 个错误码。
///
/// 设计文档 §5.2 定义了 12 个面向 Agent 的错误码。
/// 每个变体对应一个 `code()` 方法返回 `&'static str` 错误码。
use thiserror::Error;

#[derive(Error, Debug)]
pub enum McuBridgeError {
    /// 命令在当前目标状态下不可用
    #[error("command not valid in current target state")]
    State,

    /// 参数无效或缺失
    #[error("invalid or missing parameter")]
    Param,

    /// 后端通信失败
    #[error("backend communication failure")]
    Backend,

    /// 探针断连，恢复中
    #[error("probe disconnected, recovery in progress")]
    Probe,

    /// 探针恢复失败，会话即将结束
    #[error("probe recovery failed, session ending")]
    ProbeLost,

    /// Flash 操作失败
    #[error("flash operation failed")]
    Flash,

    /// 需要 DWARF 信息但不可用
    #[error("DWARF info needed but not available")]
    NoDwarf,

    /// 操作在 semihosting 模式下不支持
    #[error("operation not supported in semihosting mode")]
    NoSemihosting,

    /// Flash 断点未启用
    #[error("flash breakpoints not enabled")]
    FlashBpDisabled,

    /// Flash 断点会话次数已达上限
    #[error("flash breakpoint session limit reached")]
    FlashBpLimit,

    /// 串口操作失败
    #[error("serial port operation failed")]
    Serial,

    /// 内部错误
    #[error("internal error")]
    Internal,
}

impl McuBridgeError {
    /// 返回 JSON-Lines 协议中对应的错误码字符串。
    pub fn code(&self) -> &'static str {
        match self {
            Self::State => "E_STATE",
            Self::Param => "E_PARAM",
            Self::Backend => "E_BACKEND",
            Self::Probe => "E_PROBE",
            Self::ProbeLost => "E_PROBE_LOST",
            Self::Flash => "E_FLASH",
            Self::NoDwarf => "E_NO_DWARF",
            Self::NoSemihosting => "E_NO_SEMIHOSTING",
            Self::FlashBpDisabled => "E_FLASH_BP_DISABLED",
            Self::FlashBpLimit => "E_FLASH_BP_LIMIT",
            Self::Serial => "E_SERIAL",
            Self::Internal => "E_INTERNAL",
        }
    }
}
