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
| `--halt-on-start` | 保持暂停；同时指定 `--continue` 时优先 |
| `--sampling-interval <MS>` | 采样间隔；优先于配置的 `watch.interval_ms`，必须大于零 |

`--watch` 在 JSON 和 REPL 模式都生效。观测先注册，再按 `--continue` 启动目标和采样；没有观测时只运行目标，仍可检测停机事件。采样容量读取配置的 `watch.buffer_size`。

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
← {"id":2,"status":"ok","data":{"bp_id":0,"addr":134218240}}

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
6. `{"cmd":"buffer","args":{"since":N},"id":4}` 增量分析历史数据
7. 调整断点 → `resume` → 循环

发出 `resume` 后，可以保持 stdin 打开并只读取输出。停机和日志事件不依赖后续请求；响应通过 `id` 配对，异步消息通过 `event` 区分。每条输出都是完整 JSON 行并立即 flush；参数和状态规则以运行中的 `schema` 为准。

确认失联时会发送 `probe_recovering`（`data.code = E_PROBE`）；恢复失败会发送 `probe_lost`（`E_PROBE_LOST`）后退出。恢复成功会根据实际停机状态发送 `halted` 或 `probe_recovered`。这些事件及关联错误码也在 `schema.events` 中描述。

JSON 地址和长度必须是范围内的整数，超过 `u32` 的值或已提供但类型不正确的参数会返回 `E_PARAM`。读取历史使用 `args.since`；结束会话发送 `quit` 或关闭 stdin。

## 后端选择逻辑

1. 默认使用 **probe-rs**（纯 Rust API 直驱，支持 CMSIS-DAP / J-Link / ST-Link）
2. 指定 `--backend openocd` 或提供 `.debugger/openocd.cfg` 时启用 **OpenOCD**（TCL telnet 子进程）
3. 日志通道优先尝试 RTT → UART；只有后端实现了真实捕获能力时才尝试 Semihosting。当前 probe-rs 后端不支持 Semihosting 捕获，不会将空实现报告成可用日志通道。

## 开发与验证

本地首次编译先执行 `cargo fetch`，项目最低 Rust 版本为 1.95。

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --release --locked
cargo test --locked
cargo test --locked schema_consistency
```

默认测试通过 mock 隔离探针，不连接真实 MCU；进程测试复用生产 stdin/stdout 路径，验证输入空闲事件和退出。CI 在 Linux/Windows 运行测试，macOS 仅编译测试。CI 和 Release 构建使用提交的 `Cargo.lock`。

真实硬件验收需要另行明确 MCU、探针、后端和 ELF，验证烧录回读、断点、采样历史与资源退出。离线测试不能替代真实板证据。已经进入后端的同步硬件调用无法强制取消，停止采样会等待该调用返回。

版本变更见 [CHANGELOG.md](CHANGELOG.md)。

## 关于

MIT License. 详见 [LICENSE](LICENSE)。
