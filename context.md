# context.md - 项目核心业务上下文与共识档案

> ⓘ 本文件是项目成员与 AI 代理之间的「业务知识桥梁」。记录一切名义、战略方向、业务规则共识、需求拼图及确定不做的排除范围，以防范跨对话开发时的记忆偏差。

---

## 一、 项目背景与愿景 (Background & Vision)

- **项目一句话定位**: `mcu-bridge` 是一款面向 AI Agent 的嵌入式调试中间件。通过**缓冲区解耦 Agent 的慢思考（秒~分钟级）与 MCU 的快执行（微秒级）**，统一调试器（断点/寄存器/内存）与日志通道（RTT/UART/Semihosting）为单一 JSON-Lines 入口，让 Agent 无需手动试错编译烧录命令即可完成"烧录→采样→翻阅→分析→设断点→继续"的闭环调试。
- **核心受众**: AI Agent（首要使用者，通过 `--json` 模式交互）；嵌入式开发者（次要，通过 Human REPL 交互）
- **终极成功指标**:
  1. Agent 能独立完成"烧录固件 → 设断点/watch → 运行 → 翻阅 buffer 历史 → 分析数据趋势 → 调整断点 → 继续"的完整调试闭环，全程无需人工介入 OpenOCD/GDB 命令
  2. 调试历史数据（带 μs 时间戳的 ring buffer + 寄存器快照）在 Agent 思考期间不丢失，Agent 随时可增量查询
  3. 日志通道与调试通道在统一时间基下对齐，Agent 可跨通道关联事件

---

## 二、 关键专有名词词汇表 (Glossary of Terms)

> 任何该项目特有的、用户定义的名词缩写都必须在此注册并定义解释。

- **mcu-bridge**: 本工具的产品名称。位于 Agent/MCU 之间的调试桥梁进程。
- **DebugProbe**: 调试探针抽象层。Rust trait，定义 `attach/detach/flash/halt/resume/step/read_mem/write_mem/read_regs` 等统一接口，屏蔽底层是 probe-rs 还是 OpenOCD。
- **probe-rs**: 纯 Rust 嵌入式调试库，支持 CMSIS-DAP / J-Link / ST-Link 等探针的 API 直驱。是 DebugProbe 的**默认后端**。
- **OpenOCD**: 开源调试工具。以子进程模式启动，通过 `localhost:6666` TCL telnet 接口通信。是 DebugProbe 的**兜底后端**，覆盖 probe-rs 不支持的非主流芯片。
- **TCL 关键词匹配**: OpenOCD 后端的异步事件检测策略——接收线程监听 telnet socket，匹配关键词列表 `["halted", "breakpoint", "target halted", "target state: halted"]` 来识别断点命中。不做结构化协议解析，靠扩充列表容错版本差异。
- **DebugBuffer**: 调试缓冲层。每个 watch target 一个独立 ring buffer，定时（默认 10ms）通过 SWD 读取变量值写入 buffer。断点命中时额外记录完整寄存器快照（`bp_flag = true`），连接中断恢复后首条记录标记 `gap = true`。Agent 通过 `buffer --since N` 增量翻阅历史。
- **LogChannel**: 日志通道抽象层。Rust trait，定义 `open/read/write/close`，将 RTT / UART / Semihosting 统一为 MCU↔主机文本字节流。`SerialMonitor` 线程持有 `Box<dyn LogChannel>` 持续读取并写入日志 ring buffer。
- **RTT (SEGGER Real-Time Transfer)**: 日志通道的**一等公民**。MCU 固件在 RAM 中维护 RTT Control Block（含 ring buffer 地址和状态），调试器通过 SWD 直接读取——无需断点、无需额外引脚、MCU 侧仅 memcpy。启动时搜索特征签名 `"SEGGER RTT"` 魔数来检测。
- **UART**: 物理串口日志通道。通过主机串口（`/dev/ttyACM0` 或 `COM3`）接收 MCU 输出，支持 DMA 模式零 CPU 开销。RTT 不可用时的首选 fallback。
- **Semihosting**: ARM 调试协议——MCU 通过 `BKPT` 异常陷入调试器来输出文本。每次输出约 1-2ms 且会 halt CPU，性能差但无需任何硬件连接。三级 fallback 的最后一级，不做性能优化（协议固有缺陷）。
- **SWD (Serial Wire Debug)**: Cortex-M 调试总线协议。支持在 CPU 运行中（不 halt）读取内存，单次 32-bit 读取约 50-80μs。是 DebugBuffer 定时采样的物理基础。
- **Ring Buffer（环形缓冲区）**: 每个 watch target 的固定容量循环队列（默认 128 条），写满后覆盖最旧记录。每条记录含：全局序列号 sn、μs 时间戳 tick、采样值 val、核心号 core、断点标记 bp_flag、断连标记 gap、寄存器快照 regs（仅 bp_flag=true 时）。
- **Watch Target**: 用户通过 `watch <variable> <size>` 命令设定的数据观测目标。支持 DWARF 变量名或裸地址。每个 target 独立 ring buffer。
- **JSON-Lines**: Agent 模式的通信协议。stdin/stdout 每行一个完整 JSON 对象，无提示符。命令响应含 `status` 字段（`ok` / `halted` / `running` / `error`）。
- **Schema 协议发现**: Agent 模式下的自描述机制。Agent 发送 `{"cmd":"schema"}` 获取完整命令规格（含参数类型、valid_states、响应格式、错误码表），不依赖外部文档即可完成协议适配。`mcu-bridge` 是协议的真实来源。
- **Flash 断点**: 突破硬件断点数量限制（4-6 个）的机制——用 BKPT 指令替换 Flash 中的指令。代价：设置/清除慢（几十~几百 ms）、消耗 Flash 擦写寿命、XIP 不可用。默认关闭，需 `--enable-flash-bp` 显式开启，每会话上限 100 次。
- **芯片模板库**: 内置常用芯片（STM32/nRF/RP2040 等）的 Flash 地址/大小/RAM 地址等预设，以 TOML 文件维护。用户可通过 `mcu-bridge init --chip STM32F407VG` 自动填充。

---

## 三、 阶段性核心目标与里程碑 (Current Milestones & Goals)

### 1. 核心目标 (In-Scope Target)

实现优先级按 P0 → P3 递减：

- **[P0] DebugProbe trait + probe-rs backend 基础**: attach/detach/flash/halt/resume/step/断点/内存读写/寄存器读写。probe-rs 直驱，无外部进程依赖。
- **[P0] CLI 框架 (clap) + 顶层子命令**: `init`（生成 .debugger/chip.toml）、`flash`（烧录 ELF）、`clean`（缓存清理）、`debug`（进入调试会话）。
- **[P0] flash 子命令真实烧录**: Standalone 模式 probe-rs attach → flash → detach，`--chip`/`.debugger/chip.toml` 配置源，`--run` 自动运行，`--verify` 默认开启回读校验。
- **[P1] DebugBuffer + 定时采样 + ring buffer**: 独立线程，默认 10ms 周期通过 SWD 读取 watch target 值；断点触发时额外记录寄存器快照；连接恢复标记 gap。
- **[P1] debug 子命令 + Human REPL + --json 模式 + schema 协议发现**: 双模式界面，Human REPL 有颜色和折叠，JSON-Lines 模式完整输出。schema 命令自描述协议。
- **[P1] 探针连接自恢复**: `is_connected()` 检测 + `try_recover()` 重连（默认 3 次，间隔 500ms）+ 断点/watch 自动恢复。恢复失败时保留 buffer 数据优雅退出。
- **[P1] LogChannel trait + RttChannel**: RTT Control Block 特征签名搜索 + probe-rs RTT 封装。Channel 0 为默认终端通道。
- **[P1] UartChannel + serial read/write**: 物理串口支持，自动检测端口或用户指定。
- **[P1] serial.backend = auto 三级 fallback**: RTT → UART → Semihosting 依次尝试，全部失败则报错退出。
- **[P2] SemihostingChannel**: BKPT 异常捕获实现。Semihosting 事件触发时暂停定时采样避免 SWD 总线竞争。
- **[P2] 多核支持**: 单线程轮询所有核（SWD 总线物理串行化），DebugProbe trait 的 core 参数透传，各核独立管理断点/watch/buffer。
- **[P2] OpenOCD backend**: TCL 关键词匹配 + 进程级重启（超时杀子进程→重启→重新 attach）+ Docker CI 固化 0.12 版本。
- **[P2] 芯片模板库**: STM32/nRF/RP2040 常用系列预设。
- **[P2] 缓存管理 ~/.mcu_bridge/**: 按项目 hash + 会话时间戳组织，正常退出自动清理，异常退出保留，总大小超 512MB 按最旧优先清除。
- **[P2] Buffer 快照回放**: 支持加载历史 session 的 `session.json` + `serial.log` 缓存文件，回放 buffer 数据查看变量变化趋势。`mcu-bridge debug --replay <session_dir>` 启动只读回放模式。
- **[P3] DWARF 符号解析**: 变量名→地址、函数名→地址、文件名:行号→地址。
- **[P3] 寄存器快照 + 栈回溯**: bp_flag=true 时记录完整寄存器 + 调用栈回溯。
- **[P3] Flash 断点**: 默认关闭，需 `--enable-flash-bp` 显式开启，每会话上限 100 次，CLI 和 buffer 中区分 hw/flash 类型。

### 2. 排除范围与非目标 (Out of Scope / Non-Goals)

绝对禁止后续 Agent 在本项目中开发或建议的技术点/功能方向：

- **[排除] IDE 插件 / GUI**: 本项目仅提供 CLI + JSON-Lines 接口。不做 VS Code 插件、不做 Web 仪表盘、不做图形化调试界面。
- **[排除] MCU 侧固件库**: 不自行开发 MCU 侧日志库。RTT 通道直接复用已有的 `SEGGER_RTT.c`（BSD 许可），UART/Semihosting 使用厂商标准库即可。
- **[排除] Semihosting 性能优化**: Semihosting 的每次输出 halt CPU 是 ARM 协议固有缺陷，不做优化。只做 basic 实现并在文档中标注性能数据供用户参考。
- **[排除] 多核多线程采样**: 多核共享同一根 SWD 总线，物理上就是串行化的。拆多线程只会增加 probe-rs Session 内部 Mutex 竞争，没有收益。坚持单线程轮询。
- **[排除] OpenOCD 多版本兼容矩阵**: 以 OpenOCD 0.12 为基准。关键词匹配天然容错版本差异，不做多版本兼容测试矩阵。仅在 Docker CI 中固化单一版本。
- **[排除] GDB 协议支持**: 不做 GDB server 模式。Agent 通过 JSON-Lines 协议交互，Human 通过 REPL 交互，不走 GDB RSP 协议。
- **[排除] 实时示波器/图表**: DebugBuffer 是事后翻阅的历史数据，不做实时波形绘制/数据可视化。Agent 自行解析 JSON buffer 导出数据做离线分析。
- **[排除] 无线调试 (Wi-Fi/BLE)**: 仅支持有线探针（USB 连接的 CMSIS-DAP / J-Link / ST-Link / FTDI）。不做无线调试支持。

---

## 四、 核心业务逻辑与共识决策 (Core Business Logic & Consensus)

> 用于承载多次对话中用户敲定、对齐、或解释的技术与方案大方向，每次发生原则性共识，必须在此处更新并关联具体章节。

### 1. 架构与数据流决策

- **语言选择 Rust（非 C/C++/Python）**: 零成本抽象让 trait 天然适合多后端（probe-rs/OpenOCD、RTT/UART/Semihosting）；编译为单一二进制无运行时依赖；probe-rs 本身就是 Rust 生态，同语言调用零开销。否定了 Python（运行时依赖、性能瓶颈）和 C/C++（缺乏 trait 级别的多态抽象、内存安全风险）。

- **probe-rs 优先、OpenOCD 兜底的后端策略**: probe-rs 是纯 API 驱动无需外部进程，覆盖主流 Cortex-M（STM32/nRF/RP2040/LPC/Kinetis/ATSAM）。OpenOCD 通过子进程 + TCL telnet 接入，仅在其配置文件被显式指定或芯片不在 probe-rs 支持列表时启用。否定了"只做 OpenOCD"（失去纯 API 调用优势）和"只做 probe-rs"（丧失非主流芯片覆盖）。

- **DebugBuffer 解耦 Agent 时钟与 MCU 时钟**: 核心设计洞察——不在断点处等待 Agent（传统 GDB 模式），而是持续采集带时间戳的 ring buffer 让 Agent 事后翻阅。类似逻辑分析仪的工作方式。定时采样默认 10ms 周期、SWD 开销 4-6.4%，硬实时场景可调大到 100ms。否定了"传统断点等待模式"（Agent 思考期间 MCU 已跑过关键逻辑）。

- **RTT→UART→Semihosting 三级 fallback**: RTT 是首选（SWD 读 RAM、MCU 仅 memcpy、MHz 级吞吐），UART 是需要物理引脚的备选（DMA 下零 CPU 开销），Semihosting 是无任何硬件连接的最后兜底（每次 1-2ms、halt CPU）。auto 模式依次尝试。否定了"仅支持 RTT"（丧失兼容性）和"三者同等对待"（性能差距太大，需要优先级）。

- **JSON-Lines 而非 WebSocket/HTTP**: Agent 通过 stdin/stdout 的 JSON-Lines 协议交互，每行一个完整 JSON 对象。无需网络栈、无需端口管理、天然适合子进程模式（Agent 直接 spawn `mcu-bridge debug --json`）。schema 命令实现协议自发现，无需维护外部 API 文档。否定了 WebSocket（过度设计、需要网络栈）和 HTTP REST（不适合长连接调试会话）。

- **单线程轮询多核（非多线程并行采样）**: 多核共享同一根 SWD 总线，物理上串行化。单线程轮询所有核的所有 watch target，一轮开销约 640μs（M4+M0 各 4 个 watch），对 10ms 周期可忽略。否定了多线程方案（只增加 probe-rs Session 内部 Mutex 竞争，没有实际并行收益）。

- **OpenOCD TCL 关键词匹配而非结构化解析**: TCL 命令本身是同步的（发完等返回即可），异步事件靠接收线程监听 socket 匹配关键词列表。不解析 TCL 结构化输出，靠扩充关键词列表容错版本差异。否定了"结构化解析 TCL 协议"（OpenOCD 不同版本输出格式有差异，维护成本高）。

- **Flash 断点默认关闭 + 硬上限**: 突破硬件断点 4-6 个的限制，但代价明确——慢（Flash 扇区擦写几十~几百 ms）、耗寿命（10K-100K 次）、XIP 不可用。因此默认不开启，需用户显式 `--enable-flash-bp`，且每会话上限 100 次。在 CLI 和 buffer 中区分 hw/flash 类型。否定了"默认开启 Flash 断点"（代价太大，不应由工具替用户决定）和"不做任何限制"（防止 Agent 无节制消耗 Flash 寿命）。

- **RTT Control Block 基于 ELF 的 `.noinit` 段搜索（非固定地址扫描）**: MCU 固件中 RTT Control Block 通常放在 `.noinit` 段（复位不初始化），因此从 ELF 的 linker script 信息中提取 `.noinit` 段的起止地址范围，在该范围内搜索 `"SEGGER RTT"` 魔数。比固定地址扫描（从 0x20000000 起）更精确、更可靠，避免了漏检或多芯片地址布局差异导致的搜索盲区。代价是需要解析 ELF 的 section headers，但这在 probe-rs 中已有现成支持。否定了"固定地址扫描"方案（简单但可能漏检，不同芯片 SRAM 基址不同）。

- **异步运行时选型：`std::thread` + `mpsc`（否定 tokio）**: 线程数量静态（采样线程 + 日志线程 + 主线程共 3 个），probe-rs 使用同步 API，tokio 的 M:N 调度是过度设计。编译更快、二进制更小。否定了 tokio（编译慢、async span 在此场景无收益）。

- **错误处理分层：模块内 `thiserror` + CLI 层 `anyhow` 集中映射**: 12 个 JSON-Lines 错误码是协议层概念，与内部模块失败原因不是一对一关系。各模块用 `thiserror` 自包含，CLI 入口用 `anyhow` 集中收敛映射。未来如果重构成 workspace，各 crate 的错误枚举天然独立。否定了"全项目统一 thiserror 枚举"（模块间耦合高、错误码映射混乱）。

- **诊断日志选型：`log` + `env_logger`（否定 tracing、否定 eprintln!）**: probe-rs 内部使用 `log` crate，同生态可直接通过 `RUST_LOG=probe_rs=debug` 看到探针库内部诊断。Human REPL 模式 stderr 彩色输出，JSON 模式 stdout 协议分离。否定了 tracing（async span 优势用不上）+ 手工 eprintln!（无级别过滤、无法切换输出目标）。

- **Flash 烧录子命令策略：Standalone 即用即走模式**: `mcu-bridge flash` 使用独立临时 `ProbeRsBackend` 完成 attach → flash → detach，不与 `debug` 会话共享状态。芯片配置按 `--chip` 参数 > `.debugger/chip.toml` 优先级。烧录后默认 halt（等待用户/Agent 设断点），`--run` 参数使目标自动复位运行。校验默认开启（`--verify` true），可用 `--no-verify` 关闭。进度信息输出到 stderr。仅支持 probe-rs 后端（OpenOCD 待 P2）。否定了"依赖 debug session session 复用"（跨命令共享状态复杂）和"默认运行烧录后自动运行"（开发调试场景需要 halt 检查）。已对齐归档于 [user_plan/flash-probe-rs/flash-probe-rs.md](user_plan/flash-probe-rs/flash-probe-rs.md)。

### 2. 核心功能流程定义

- **Agent 标准调试闭环流程**:
  1. `mcu-bridge debug --elf fw.elf --json` 启动会话 → 收到 `{"status":"attached",...}`
  2. `{"cmd":"schema"}` 获取协议自描述
  3. 按 valid_states 选择可用命令：设断点 (`break`)、设数据观测 (`watch`)、全速运行 (`continue`)
  4. 定时采样线程持续写入 ring buffer，Agent 侧可 `{"cmd":"sleep","ms":2000}` 等待采样窗口
  5. 断点命中时自动 halt，buffer 中对应记录 `bp_flag=true` + 寄存器快照
  6. Agent 发送 `{"cmd":"buffer","since":N}` 增量获取采样历史，离线分析数据趋势
  7. 基于分析结果调整断点/watch → `continue` → 循环

- **探针断连恢复流程**:
  1. 每次操作前 `is_connected()` 检测 → 失联则进入 RECOVERING 状态
  2. `try_recover()` 重试最多 3 次（间隔 500ms）
  3. 成功 → 自动恢复所有断点/watchpoint，buffer 下一条记录 `gap=true`
  4. 失败 → 会话优雅退出，保留 `~/.mcu_bridge/<hash>/<timestamp>/` 下的 buffer 快照和串口日志

- **日志通道 auto 检测流程**:
  1. 搜索 RAM 中 RTT Control Block 特征签名 `"SEGGER RTT"` → 找到 → RttChannel
  2. 未找到 → UART 自动检测端口 → 找到 → UartChannel
  3. 未找到 → 启用 Semihosting → SemihostingChannel
  4. 全部失败 → `{"status":"error","code":"E_SERIAL",...}` 退出

### 3. 待确认 / 待测试事项 `[待确认/Draft]`

> ⓘ 以下事项尚需实测验证，后续 Agent 在实施到对应阶段时应优先处理。

- **[待测试] Semihosting 在 probe-rs 中的实际捕获延迟**: 设计文档估计每次 1-2ms，P2 阶段需实测后量化，作为用户文档的性能参考数据。届时将正式数据写入文档。

---

## 五、 上下文维护与变更日志 (Maintenance History)

> 任何人（含 Agent 与人类用户）只要修改了本文件，必须在此按时间倒序追加一条变更声明。
>
> **写入规则**: 仅在有新的共享知识产生时追加（新术语、新架构决策、新排除范围）。单纯的 bug 修复、特性实现、重构等 git 已经记录的变更——不需要在 context.md 中重复登记。context.md 的变更日志仅记录**影响 Glossary、架构决策、Non-Goals 的知识级变更**。

- **[2026-06-03]**: 归档 ci-cd 需求。验收通过：ci.yml 三平台 matrix（Linux/Win/macOS）+ Swatinem/rust-cache@v2 + release.yml tag 自动发布 + Cargo.lock 生成。提炼 2 条刺卡经验反哺 AGENTS.md：(1) 二进制项目的 Cargo.lock 必须提交以保障 CI 缓存命中率；(2) upload-artifact/download-artifact 跨 job 传递多平台二进制模式。目录已归档至 [user_plan/archive/ci-cd/](user_plan/archive/ci-cd/)。(By Agent - Archive-and-Summary)
- **[2026-06-03]**: 归档 P0 (p0-probe-cli)。验收通过：probe-rs backend 12 方法实现（attach/detach/flash/halt/resume/step/core_count/active_core/set_breakpoint/clear_breakpoint/read_mem/write_mem/read_regs）+ CLI init/clean/flash 三子命令 + 10 mock 测试全绿 + cargo check/fmt/test 全量通过。提炼 2 条刺卡经验反哺 AGENTS.md：(1) probe-rs API 侦察策略——让编译器给出修正提示而非提前 grep 源码；(2) Rust 借用检查器模式——先改 self 再借子对象。目录已归档至 [user_plan/archive/p0-probe-cli/](user_plan/archive/p0-probe-cli/)。(By Agent - Archive-and-Summary)
- **[2026-06-03]**: 归档 proj-skeleton 需求。通过 Archive-and-Summary 验收：task.md 全部 24 项标记 completed、目录物理搬移至 [user_plan/archive/proj-skeleton/](user_plan/archive/proj-skeleton/)。提炼本阶段 3 条刺卡经验反哺 AGENTS.md：(1) 首次 cargo fetch 解耦网络编译，(2) Windows 非 rustup 环境 clippy-driver 可用性检查，(3) 骨架期 #![allow(dead_code)] 的生命周期管理。(By Agent - Archive-and-Summary)
- **[2026-06-03]**: 项目骨架搭建完成。基于 Understanding (Grill Me) 三轮烤问确立三大技术决策：(1) 异步运行时 std::thread+mpsc，(2) 错误处理 thiserror+anyhow 分层，(3) 诊断日志 log+env_logger。决策已归档到 §四.1 架构决策。项目骨架含 Cargo.toml（10个依赖）、18个 .rs 源文件（trait/struct/enum 骨架）、rustfmt.toml、.gitignore、.github/workflows/ci.yml。cargo check + cargo fmt + cargo run --help 全量通过。需求规格书/任务清单/编码红线归档于 user_plan/proj-skeleton/。(By Agent - Understanding/Code-Spec)
- **[2026-06-03]**: 落实三项待确认事项的用户决策：(1) RTT CB RAM 搜索策略确认为基于 ELF `.noinit` 段，移入 §四.1 架构决策；(2) Buffer 快照回放确认为需支持，追加为 §三.1 P2 核心目标；(3) Semihosting 延迟标记调整为 [待测试]，保留在 §四.3。同步更新设计文档 §11。(By Agent - Context-of-User)
- **[2026-06-03]**: 初始化本档案。从 `嵌入式调试软件.md` 设计文档中提炼项目一句话定位、核心受众、专有名词词汇表（mcu-bridge/DebugProbe/DebugBuffer/LogChannel/RTT/SWD/ring buffer/watch target/JSON-Lines/schema 协议发现/Flash 断点等 16 项）、P0-P3 实施路线里程碑、排除范围（不做 IDE 插件/GUI/GDB 协议/无线调试等 8 项）、架构决策与权衡理由（Rust 选型/probe-rs 优先/三级 fallback/单线程多核/关键词匹配/Flash 断点默认关闭等 8 项）、3 项待确认事项。(By Agent - Context-of-User)
- **[2026-06-04]**: 伴随需求 [user_plan/flash-probe-rs/flash-probe-rs.md](user_plan/archive/flash-probe-rs/flash-probe-rs.md) 探底自检同步更新 Glossary 与 Non-Goals 属性。新增决策：Flash 烧录 Standalone 模式、芯片配置优先级规则、烧录后默认 halt/--run 运行、--verify 默认开启回读校验、进度 stderr 输出、仅 probe-rs 后端。已归档到 §四.1 架构决策。(By Agent - Understanding/Context-of-User)
- **[2026-06-04]**: 验收并通过功能 [user_plan/archive/flash-probe-rs/](user_plan/archive/flash-probe-rs/)，实测 STM32F411RE 硬件烧录通过。提炼 2 条刺卡反哺 AGENTS.md：(1) probe-rs 芯片名精确性约束——模板 name 必须使用 probe-rs 识别的精确 target 名称，用户输入应透传；(2) 用户审批权原则——禁止擅自提交，所有 git push 前须获用户批准。AGENTS.md 已增补 §一.4 用户审批权原则 和 §二.2 probe-rs 芯片名精确性经验。目录已物理归档。(By Agent - Archive-and-Summary)
- **[2026-06-06]**: 验收并通过功能 [user_plan/archive/debug-round2/](user_plan/archive/debug-round2/)（评判本周期无痛点，宪法简炼合用不变）。(By Agent - Archive-and-Summary)
- **[2026-06-05]**: 开发并交付 OpenOCD 兜底烧录后端。通过 Understanding (Grill Me) 四轮决策对齐：后端选择 `[C]`（CLI>TOML>缺省 probe-rs）、配置文件 `[C]`（--openocd-cfg > TOML > .debugger/openocd.cfg 兜底）、烧录协议 `[A]`（program 一行命令）、进程生命周期 `[A]`（极简模式）。实现 `src/probe/openocd.rs` 的 `attach/flash/resume/detach/Drop` 五个方法，`src/cli/flash.rs` 的 `create_backend()` 工厂 + `handle()` 路由化。17/17 测试通过。新增决策：OpenOCD Standalone flash 极简生命周期。已物理归档至 [user_plan/archive/flash-openocd-backend/](user_plan/archive/flash-openocd-backend/)。(By Agent - Understanding + Archive-and-Summary)
- **[2026-06-05]**: 探底对齐 Debug REPL 需求（Round 1）。通过 Understanding (Grill Me) 四轮决策对齐：交付范围 `[C]`（先 Human REPL 再 Agent 模式）、命令集 `[C]`（结构化 enum + 8 命令 + help）、架构模式 `[B]`（DebugRepl 结构体 + 方法拆分）、状态持有 `[A]`（Session 扩展持有 Box\<dyn DebugProbe\>）。新增决策：最小可用 REPL 设计——9 条命令、状态守卫机制、parse/execute 分离。需求文档已落盘至 [user_plan/archive/debug-repl/debug-repl.md](user_plan/archive/debug-repl/debug-repl.md)。(By Agent - Understanding)
- **[2026-06-05]**: 实现并交付 Debug REPL (Round 1)。基于 code-spec 生成的 19 项任务全部完成：Session 扩展（attach/detach）、Command 枚举（parse/valid_states）、DebugRepl 结构体（new/run/9命令方法）、handle() 集成。最终状态：46/46 测试通过（含 28 个新增）+ cargo fmt 零差异 + cargo check 零 warning。已物理归档至 [user_plan/archive/debug-repl/](user_plan/archive/debug-repl/)。(By Agent - Implementation)
