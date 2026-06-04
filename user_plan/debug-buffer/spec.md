# spec.md — DebugBuffer 编码红线契约

> ⓘ 本文件是本次编码的"保险圈与硬约束控制中心"。

---

## 一、 禁止触碰的红线

- **红线 1 [main.rs 冻结]**: `src/main.rs` 不动
- **红线 2 [Cargo.toml 冻结]**: 不新增外部依赖
- **红线 3 [CLI Commands 枚举完整保留]**: Init/Flash/Clean/Debug 四个变体不可改
- **红线 4 [已有测试全量保护]**: 原 59 个测试不得被修改或删除

## 二、 实现红线

- **红线 5 [try_lock 优先]**: Sampler 使用 `lock()` 而非 `try_lock()`，后端操作失败则跳过本轮
- **红线 6 [wait_for_core_halted 短超时]**: 断点检测使用 1ms 超时，非阻塞
- **红线 7 [采样线程不可 panic]**: 所有 `backend.read_mem` 失败只打 warn，不 panic
- **红线 8 [watch size 限制]**: size 只允许 1/2/4/8，其余报错

## 三、 验收核对

- [ ] 原 59 测试 + 11 新测试全部通过
- [ ] cargo fmt + cargo check 零差异
- [ ] main.rs / Cargo.toml 未修改
