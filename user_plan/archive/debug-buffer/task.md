# task.md — DebugBuffer 精细化函数级开发任务清单

> ⓘ 本文件是实现"定时采样 + ring buffer + 变量观测"的 AI 代理的核心执行手记。

---

- **整体进度状态**: `completed`

## 一、 基础设施层改动

- [x] **Task 1.1: Session::backend 改为 Arc<Mutex<>>**
- [x] **Task 1.2: 更新所有 backend.lock() 调用点**

## 二、 ProbeRsBackend is_halted() 真实实现

- [x] **Task 2.1: 新增 target_halted 缓存字段**
- [x] **Task 2.2: is_halted() 改 &mut self + 返回缓存值**

## 三、 Sampler + DebugBuffer 实现

- [x] **Task 3.1: DebugBuffer 增强方法**
- [x] **Task 3.2: Sampler 结构体 + run()**

## 四、 CLI/JSON 集成

- [x] **Task 4.1: Command 新增 Watch/Buffer 变体 + parse + valid_states**
- [x] **Task 4.2: DebugRepl 新增 cmd_watch/cmd_buffer + resume/halt 采样集成**
- [x] **Task 4.3: JsonSession execute_json 支持 watch/buffer + schema**
- [x] **Task 4.4: handle() 集成 --watch 和 --sampling-interval**

## 五、 全局验证

- [x] **Task 5.1: 70/70 测试通过 + fmt + check**
