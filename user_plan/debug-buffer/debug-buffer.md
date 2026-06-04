# 需求规格说明书：DebugBuffer — 定时采样 + ring buffer + 变量观测

> ⓘ 本文档已经过 "Understanding (Grill Me)" 技能对齐，记录了 4 轮决策的完整共识树。本文件已归档于 [user_plan/debug-buffer/debug-buffer.md](debug-buffer.md)。实现该特性的后续 Agent 必须全盘以此文档为验收终点。

---

## 一、 功能概览与基本愿景 (User Story & Goal)

- **新功能背景**: `mcu-bridge` 已具备完整的 Human REPL + Agent JSON-Lines 双模式调试能力（设断点/单步/读寄存器/读内存）。但所有操作都是"即时"的——Agent 必须持续在线才能获取数据。缺少核心差异化能力：**缓冲区解耦 Agent 慢思考与 MCU 快执行**。没有 DebugBuffer，Agent 无法完成"烧录→采样→翻阅→分析→设断点→继续"的完整闭环。
- **用户故事 (User Story)**:
  - 作为一名 AI Agent（首要受众），我想要在 `resume` 后让 `mcu-bridge` 自动以 10ms 周期采样我指定的内存变量，并在断点命中后让我通过 `{"cmd":"buffer","args":{"since":N}}` 增量翻阅采样历史，以便在 Agent 思考期间不丢失 MCU 运行的任何关键数据。
  - 作为一名嵌入式开发者，我想要在 Human REPL 中用 `watch 0x20000000:4:counter` 命令启动变量观测，用 `buffer` 命令查看采样历史，以便在调试会话中追踪变量变化趋势。
- **关联已有的技术链**:
  - `src/buffer/mod.rs` — 已有 `Sample`/`WatchTarget`/`DebugBuffer` 结构体骨架，`push_sample()` 已实现
  - `src/buffer/serial.rs` — `SerialMonitor` 占位结构体
  - `src/session.rs` — `Session` 持有 `backend: Box<dyn DebugProbe>`，需改为 `Arc<Mutex<...>>`
  - `src/cli/debug.rs` — `Command` enum 需新增 `Watch`/`Buffer` 变体；`execute()` 需实现
  - `src/cli/json_session.rs` — `execute_json()` 需新增 `watch`/`buffer` 命令处理
  - `src/probe/mod.rs` — `DebugProbe` trait 签名冻结
  - `src/probe/probe_rs.rs` — `is_halted()` 当前返回 false（stub），需实现真实检测
  - `src/config.rs` — `WatchConfig` 已有（`interval_ms`/`buffer_size`）

### 本轮不做

| 排除项 | 说明 |
|--------|------|
| ❌ LogChannel (RTT/UART/Semihosting) | 独立的 P1 特性，下一轮 |
| ❌ 探针连接自恢复 | 独立的 P1 特性 |
| ❌ --serial-port 串口配置 | LogChannel 特性的一部分 |
| ❌ --enable-flash-bp Flash 断点 | P3 特性 |
| ❌ DWARF 符号解析 | P3 特性 |

---

## 二、 极限对齐的业务流程 (Consensus Flow & Sequence)

### 共识决策树

| 决策节点 | 选定方案 | 替代方案否决理由 |
|---------|---------|----------------|
| Q1: 采样线程架构 | `[B]` Arc\<Mutex\> 共享 backend | A(主循环内联) Agent sleep 不采样；C(分离) 重构量过大 |
| Q2: 线程生命周期 | `[A]` resume 启 / halt 停 | B(常驻线程) Halted 态浪费 CPU；C(channel 信号) 过度设计 |
| Q3: --watch 格式 | `[C]` `地址:大小:标签` | A(纯地址) 缺标签；B(DWARF 变量名) 需要 P3 |
| Q4: 断点检测 | `[A]` 采样线程内嵌 wait_for_core_halted 短超时 | B(独立等待线程) 3 线程竞争锁 |

### 1. Agent JSON-Lines 标准流 (Happy Path)
1. 启动：`mcu-bridge debug --elf fw.elf --chip STM32F407VG --json --watch 0x20000000:4:counter`
2. schema → break → resume → 自动采样线程
3. 采样线程每 10ms 读取变量 → ring buffer
4. 断点命中 → halted 事件 → buffer 查询
5. 分析后续步骤

### 2. 异常与阻断流
- watch 地址/大小错误 → 报错退出
- 采样断连 → 采样线程退出，E_PROBE 事件
- read_mem 失败 → skip 本轮采样

---

## 三、 架构设计方案

### 3.1 Session 并发改造
`Session::backend` 从 `Box<dyn DebugProbe>` → `Arc<Mutex<Box<dyn DebugProbe>>>`
所有 `.backend.method()` → `.backend.lock().unwrap().method()`
新增 `shared_backend()` 返回 clone 供采样线程持有。

### 3.2 ProbeRsBackend::is_halted() 状态缓存
新增 `target_halted: bool` 字段
`halt()` 设 true，`resume()` 设 false
采样线程调用 `wait_for_core_halted(1ms)` 成功后设 true

### 3.3 Sampler 结构体
持有 `Arc<Mutex<Box<dyn DebugProbe>>>` + `Arc<RwLock<DebugBuffer>>` + `Arc<AtomicBool>` stop_flag
run() 循环：sleep → lock → read_mem all targets → push_sample → is_halted 检测 → break/unlock

### 3.4 Command 扩展
`Watch { addr: u32, size: u32, label: Option<String> }` — halted 态
`Buffer { since: Option<u64>, watch_id: Option<usize> }` — 任何态

---

## 四、 受影响文件清单

| 文件 | 改动 | 说明 |
|------|------|------|
| src/session.rs | 🟡 修改 | backend 类型改为 Arc\<Mutex\>；新增 shared_backend() |
| src/probe/mod.rs | 🟡 修改 | is_halted() 签名改 &mut self |
| src/probe/probe_rs.rs | 🟢 实现 | target_halted 缓存 + 真实验证 |
| src/buffer/mod.rs | 🟢 增强 | Sampler + parse_watch_spec + add_target + get_samples + summarize |
| src/cli/debug.rs | 🟢 实现 | Watch/Buffer 变体 + 采样线程启停 |
| src/cli/json_session.rs | 🟢 实现 | watch/buffer 命令 + schema |

---

## 五、 验收断言 DoD

- [ ] Arc\<Mutex\> 改造完成，编译通过
- [ ] `--watch` 和 REPL `watch` 命令可用
- [ ] 采样线程 resume 启 / halt 停
- [ ] 定时采样每 interval_ms 读取所有 target
- [ ] 断点检测 + halted 事件推送
- [ ] buffer 查询支持 --since 增量
- [ ] 原有 59 测试全绿
- [ ] fmt + clippy 零差异
