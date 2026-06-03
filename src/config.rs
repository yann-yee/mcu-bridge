/// 配置类型定义 — 映射设计文档 §6 的 TOML 配置结构。
///
/// 所有 struct 均派生 `Serialize`/`Deserialize`，
/// 可直接通过 `toml::from_str` 从 `.debugger/chip.toml` 反序列化。
use serde::{Deserialize, Serialize};

/// 芯片配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipConfig {
    /// 芯片名称，如 "STM32F407VG"
    pub name: String,
    /// 架构，如 "cortex-m4"
    pub architecture: String,
    /// Flash 基址
    pub flash_base: u32,
    /// Flash 大小 (bytes)
    pub flash_size: u32,
    /// RAM 基址
    pub ram_base: u32,
    /// RAM 大小 (bytes)
    pub ram_size: u32,
}

/// Flash 扇区/段定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashSection {
    pub name: String,
    pub addr: u32,
    pub len: u32,
}

/// Flash 操作参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashOpts {
    pub base: u32,
    pub size: u32,
    pub sections: Vec<FlashSection>,
    /// 烧录后是否执行校验
    pub verify: bool,
}

/// 调试器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebuggerConfig {
    /// 探针类型: "stlink-v2" | "jlink" | "cmsis-dap" | "ftdi"
    pub probe: String,
    /// 调试接口: "swd" | "jtag"
    pub interface: String,
    /// SWD/JTAG 时钟频率 (kHz)
    pub speed_khz: u32,
    /// 后端: "probe-rs" | "openocd"
    pub backend: String,
}

/// 串口/日志通道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConfig {
    /// 日志后端: "rtt" | "uart" | "semihosting" | "auto"
    pub backend: String,
    /// 串口端口: "auto" | "/dev/ttyACM0" | "COM3"
    pub port: String,
    /// 波特率 (仅 UART)
    pub baudrate: u32,
    /// RTT 通道编号 (0~15)
    pub rtt_channel: usize,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            backend: "auto".into(),
            port: "auto".into(),
            baudrate: 115_200,
            rtt_channel: 0,
        }
    }
}

/// 采样观测配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    /// 定时采样周期 (ms)，默认 10
    pub interval_ms: u64,
    /// 每个 watch target 的 ring buffer 容量，默认 128
    pub buffer_size: usize,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            interval_ms: 10,
            buffer_size: 128,
        }
    }
}

/// 探针断连恢复配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// 最大重试次数，默认 3
    pub max_retries: u32,
    /// 重试间隔 (ms)，默认 500
    pub retry_delay_ms: u64,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 500,
        }
    }
}

/// Flash 断点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashBpConfig {
    /// 是否启用 Flash 断点，默认关闭
    pub enabled: bool,
    /// 单次会话最大 Flash 断点设置次数，默认 100
    pub max_per_session: u32,
}

impl Default for FlashBpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_per_session: 100,
        }
    }
}

/// OpenOCD 后端专用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenOcdConfig {
    /// OpenOCD 配置文件路径
    pub cfg_file: String,
    /// 额外命令行参数
    pub extra_args: Vec<String>,
}

/// 应用顶层配置 — 对应 `.debugger/chip.toml`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub chip: ChipConfig,
    pub debugger: DebuggerConfig,
    pub flash: FlashOpts,
    #[serde(default)]
    pub serial: SerialConfig,
    #[serde(default)]
    pub watch: WatchConfig,
    #[serde(default)]
    pub recovery: RecoveryConfig,
    #[serde(default)]
    pub flash_bp: FlashBpConfig,
    /// 仅 backend = "openocd" 时生效
    pub openocd: Option<OpenOcdConfig>,
}
