# 需求规格说明书：OpenOCD 兜底烧录后端

> ⓘ 本文档已经过 "Understanding (Grill Me)" 技能对齐，记录了逻辑共识。本文件已于 [user_plan/flash-openocd-backend/flash-openocd-backend.md](flash-openocd-backend.md) 归档。实现该功能的后续 Agent 必须全盘以此文档为验收终点。

---

## 一、 功能概览与基本愿景 (User Story & Goal)

- **新功能背景**: `mcu-bridge flash` 子命令当前仅支持 probe-rs 后端。对于 probe-rs 不支持的芯片（非主流 Cortex-M、RISC-V、Xtensa 等），用户无法通过单一工具完成烧录，需手动编写 OpenOCD 命令。
- **用户故事 (User Story)**: 作为一名嵌入式开发者或 AI 调试 Agent，我想要 `mcu-bridge flash` 支持通过 `--backend openocd` 参数切换到 OpenOCD 后端烧录固件，以便于在 probe-rs 不支持的目标芯片上也能使用统一的 CLI 界面完成烧录。
- **关联已有的技术链**:
  - `src/probe/openocd.rs`：OpenOCD backend 骨架（全部 `todo!()`）→ 需要实现 `attach/flash/detach/resume` 四个方法
  - `src/cli/flash.rs`：`handle()` 函数硬编码 `ProbeRsBackend` → 需要后端选择工厂
  - `src/cli/mod.rs`：`Flash` 变体需新增 `--backend`、`--openocd-cfg` 参数
  - `src/main.rs`：`Flash` 模式匹配需解构新字段
  - `src/config.rs`：已有 `OpenOcdConfig` + `AppConfig.openocd` 字段，无需新增

---

## 二、 极限对齐的业务流程 (Consensus Flow & Sequence)

> 记录烤问阶段彻底敲定的顺畅流与异常阻断流。

### 共识决策树

| 决策节点 | 选定方案 | 替代方案否决理由 |
|---------|---------|----------------|
| Q1: 后端选择策略 | `[C]` —backend CLI > TOML > 缺省 probe-rs | B(自动兜底) 增加用户等待时间不可控 |
| Q2: 配置文件路径 | `[C]` —openocd-cfg > TOML > .debugger/openocd.cfg 兜底 | A(纯CLI) 不友好；B(纯TOML) 心智负担高 |
| Q3: 烧录协议 | `[A]` program 一行命令 | B(逐条TCL) 代码量大、无必要 |
| Q4: 进程生命周期 | `[A]` 极简: spawn→wait→program→exit/kill→wait | B/C(超时重试) Standalone一次性操作不需要 |

### 1. 标准顺畅流 (Happy Path)

1. 用户执行 `mcu-bridge flash --elf fw.elf --chip STM32F411RE --backend openocd --openocd-cfg .debugger/stlink-v2.cfg`
2. `create_backend()` 根据 `--backend openocd` 创建 `OpenOcdBackend`（cfg_path 已存储）
3. `attach()`：spawn OpenOCD → 轮询 TCP 6666（最多 5s）→ 连接 TcpStream
4. `flash()`：发送 `program <elf_path> verify` → 等待提示符 → 校验输出不含 error
5. (若 `--run`) `resume()`：发送 `reset` → 芯片复位运行
6. `detach()`：发送 `exit` → 回收子进程
7. 输出 `[OK] flash complete`

### 2. 异常与阻断流 (Exception Handlings)

| 失败场景 | 错误信息 | 错误码 |
|---------|---------|--------|
| OpenOCD 可执行文件找不到 | `"OpenOCD not found: ..."` | E_BACKEND |
| OpenOCD 启动超时（5s） | `"OpenOCD failed to start (timeout after 5s)"` | E_BACKEND |
| 配置文件不存在 | `"OpenOCD cfg file not found: ..."` | E_PARAM |
| flash 失败（program 含 error/failed） | `"OpenOCD flash failed: ..."` | E_FLASH |
| TCL 命令超时（5s read_timeout） | `"OpenOCD command timeout ..."` | E_BACKEND |
| 未知后端类型 | `"unknown backend 'xxx'. Supported: probe-rs, openocd"` | E_PARAM |

---

## 三、 后端实现契约 (Backend Implementation Contract)

### 3.1 `OpenOcdBackend` 结构

```rust
pub struct OpenOcdBackend {
    process: Option<Child>,      // openocd 子进程
    telnet: Option<TcpStream>,   // TCP localhost:6666
    cfg_path: Option<String>,    // 配置文件路径
}
```

### 3.2 方法实现

| `DebugProbe` 方法 | 实现策略 | 状态 |
|-------------------|---------|------|
| `attach()` | spawn OpenOCD → 轮询 TCP 6666 5s | ✅ P0 |
| `detach()` | send `exit` → wait 轮询 5s → kill 保底 | ✅ P0 |
| `flash()` | send `program <elf> verify` → check response | ✅ P0 |
| `resume()` | send `reset` → check response | ✅ P0 |
| `is_connected()` | return `telnet.is_some()` | ✅ P0 |
| `core_count()` | return 1 | ✅ P0 |
| `active_core()` | return 0 | ✅ P0 |
| 其余 11 个方法 | `anyhow::bail!("P2: ...")` | 🟡 P2 todo! |

### 3.3 后端选择工厂 `create_backend()`

优先级链：`--backend` CLI 参数 > `.debugger/chip.toml` 的 `[debugger].backend` > 缺省 `"probe-rs"`

```rust
fn create_backend(args: &FlashArgs) -> anyhow::Result<Box<dyn DebugProbe>>;
```

### 3.4 CLI 参数变更

`Commands::Flash` 新增：

| 参数 | 类型 | 说明 |
|------|------|------|
| `--backend` | `Option<String>` | `"probe-rs"` \| `"openocd"` |
| `--openocd-cfg` | `Option<String>` | OpenOCD 配置文件路径 |

---

## 四、 测试策略 (Test Strategy)

### 单元测试清单

| 测试名 | 文件 | 断言 |
|--------|------|------|
| `test_openocd_creation` | `probe/openocd.rs` | 后端创建后 `not connected`, `core_count() == 1` |
| `test_openocd_attach_no_cfg` | `probe/openocd.rs` | 配置文件不存在 → Err 含 "cfg file not found" |
| `test_flash_backend_probe_rs_default` | `cli/flash.rs` | 无 `--backend` → `create_backend()` 返回 Ok |
| `test_flash_backend_openocd_no_cfg` | `cli/flash.rs` | `--backend openocd` + 无 cfg → handle() Err |
| `test_flash_backend_unknown` | `cli/flash.rs` | `--backend invalid` → Err 含 "unknown backend" |

### 红线契约 (不触及范围)

- [ ] DebugProbe trait 签名不修改
- [ ] `src/probe/probe_rs.rs` 不修改
- [ ] `Cargo.toml` 无新增依赖
- [ ] OpenOCD/log/buffer/session 模块只修改 `openocd.rs`
- [ ] CLI Commands 枚举 4 个变体完整保留
- [ ] 已有 10 个 probe-rs 测试不变 + 3 个 flash 测试不变

---

## 五、 验收断言与 Definition of Done

- [ ] **1. CLI 参数可见性**: `cargo run -- flash --help` 输出包含 `--backend` 和 `--openocd-cfg`
- [ ] **2. 标准链路单元测试**: 17 个测试全部通过（含 2 个新增 OpenOCD + 3 个新增路由），可跳过硬件依赖 flaky 测试
- [ ] **3. 格式化**: `cargo fmt --all -- --check` 零差异
- [ ] **4. 构建**: `cargo check` 编译通过，零 warning
- [ ] **5. 错误隔离**: 无效后端 → `"unknown backend"`，无 cfg 文件 → `"cfg file not found"`，ELF 不存在 → `"ELF file not found"`
- [ ] **6. 物理归档**: 本需求文档 + task.md + spec.md 三件套完整落盘 `user_plan/flash-openocd-backend/`
- [ ] **7. context.md 同步**: Glosary 与决策日志已追加
