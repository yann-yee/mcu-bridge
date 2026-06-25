# mcu-bridge

[![CI](https://github.com/yann-yee/mcu-bridge/actions/workflows/ci.yml/badge.svg)](https://github.com/yann-yee/mcu-bridge/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/mcu-bridge.svg)](https://crates.io/crates/mcu-bridge)

**面向 AI Agent 的嵌入式调试中间件。**

通过缓冲区解耦 Agent 的慢思考（秒~分钟级）与 MCU 的快执行（微秒级），统一调试器与日志通道为单一 JSON-Lines 入口。

## 安装

```bash
# 方式一：从 crates.io 安装（需 Rust 工具链）
cargo install mcu-bridge

# 方式二：从 GitHub Releases 下载预编译二进制
# https://github.com/yann-yee/mcu-bridge/releases
```

## 快速开始

三步点亮一个 LED：

```bash
# 1. 初始化芯片配置
mcu-bridge init --chip STM32F411RE --debugger stlink-v2

# 2. 烧录固件并自动运行
mcu-bridge flash --elf fw.elf --run

# 3. 启动调试会话
mcu-bridge debug --elf fw.elf
```

## 架构

```
CLI / Agent (JSON-Lines)
       │
  ┌────▼────┐
  │ Session │  状态机管理（Halted / Running / Recovering）
  └────┬────┘
       │
  ┌────▼──────────┐            ┌────────────────────┐
  │  DebugBuffer  │            │    LogChannel      │
  │  (ring buffer │            │  (RTT/UART/Semi)   │
  │   + 采样线程) │            │  + SerialMonitor   │
  └────┬──────────┘            └───────┬────────────┘
       │                               │
  ┌────▼───────────────────────────────▼────┐
  │              DebugProbe                  │
  │  统一 trait: attach/detach/flash/halt/  │
  │  resume/step/断点/内存/寄存器/watchpoint │
  └────┬──────────────────────┬─────────────┘
       │                      │
  ┌────▼────┐          ┌─────▼──────┐
  │probe-rs │          │  OpenOCD   │
  │ backend │          │  backend   │
  └─────────┘          └────────────┘
```

## CLI 命令

| 命令 | 用途 | 示例 |
|------|------|------|
| `init` | 初始化芯片配置 | `mcu-bridge init --chip STM32F411RE` |
| `flash` | 烧录 ELF 固件 | `mcu-bridge flash --elf fw.elf --run` |
| `clean` | 清理缓存 | `mcu-bridge clean --all` |
| `debug` | 启动调试会话 | `mcu-bridge debug --elf fw.elf` |
| `doctor` | 非写入式连接/状态诊断 | `mcu-bridge doctor --json` |

### debug 子命令关键参数

| 参数 | 说明 |
|------|------|
| `--elf <PATH>` | ELF 文件路径（必需） |
| `--json` | Agent JSON-Lines 模式（默认 Human REPL） |
| `--no-flash` | 跳过烧录步骤 |
| `--no-verify` | 关闭 Flash 回读校验（默认开启） |
| `--chip <NAME>` | 芯片型号（默认从 `.debugger/chip.toml` 读取） |
| `--backend <NAME>` | 强制指定后端：`probe-rs` \| `openocd` \| `auto` |
| `--openocd-cfg <PATH>` | OpenOCD 配置文件路径 |
| `--break <ADDR>` | 启动后立即设断点（可重复） |
| `--watch <ADDR>:<SIZE>` | 启动后立即设数据观测（可重复） |
| `--continue` | 启动后立即全速运行 |

### REPL 命令（Human 模式）

```
halt              Pause target execution
resume, go        Resume target execution (starts sampler)
step, s           Single-step (halted)
break <addr>, b   Set hardware breakpoint (halted)
regs, registers   Show core registers (halted)
mem <addr> <len>  Read memory (halted)
watch <a>:<s>[:l] Add watch target (halted)
buffer [since]    Show sampling history
serial [since]    Show log history
info <subcmd>     Query DWARF info (functions/variables/symbol)
status, st        Show session status
help, h, ?        Show this help
quit, exit, q     Exit debug session
```

## JSON-Lines Agent 协议

Agent 通过 stdin/stdout 每行一个 JSON 对象与 `mcu-bridge` 通信：

```
→ {"cmd":"schema","id":1}
← {"id":1,"status":"ok","data":{"commands":[...],"error_codes":{...}}}

→ {"cmd":"break","args":{"addr":134218240},"id":2}
← {"id":2,"status":"ok","data":{"id":0,"addr":134218240}}

← {"event":"attached","data":{"chip":"STM32F411RE","core_count":1,"backend":"probe-rs","state":"Halted"}}

→ {"cmd":"resume","id":3}
← {"id":3,"status":"ok","data":{"status":"running","sampling":false,"sampling_interval_ms":10}}

← {"event":"halted","data":{"pc":134218242,"core":0,"function":"main+0x02"}}

→ {"cmd":"buffer","args":{"since":0},"id":4}
← {"id":4,"status":"ok","data":{"samples":[...],"count":5}}
```

Agent 标准调试闭环：
1. `mcu-bridge debug --elf fw.elf --json` → 收到 `{"event":"attached",...}`
2. `{"cmd":"schema"}` 获取协议自描述
3. 设断点、watch target → `resume`
4. 采样线程持续写入 ring buffer
5. 断点命中 → `{"event":"halted",...}` 含函数名
6. `{"cmd":"buffer","since":N}` 增量分析历史数据
7. 调整断点 → `resume` → 循环

## 后端选择逻辑

1. 默认使用 **probe-rs**（纯 Rust API 直驱，支持 CMSIS-DAP / J-Link / ST-Link）
2. 指定 `--backend openocd` 或提供 `.debugger/openocd.cfg` 时启用 **OpenOCD**（TCL telnet 子进程）
3. 日志通道优先级：RTT → UART → Semihosting（`serial.backend = auto`）

## 关于

MIT License. 详见 [LICENSE](LICENSE)。
