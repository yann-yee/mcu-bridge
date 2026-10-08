# Changelog

## 1.0.2

Agent 调试闭环可靠性更新。本机离线验收已通过；真实硬件验收仍待确定设备组合。

### Added

- 结构化 `flash --json` 阶段/后端尝试结果和 `doctor --json` 诊断。
- debug 状态和停机输出中的寄存器/故障快照。
- 基于 mock 的 flash/doctor 工作流、启动观测与线程退出回归。
- 复用生产 stdin/stdout 路径的离线进程测试。

### Fixed

- JSON 会话在输入空闲时仍检测停机并推送日志，无观测目标时也查询实际停机状态。
- 确认失联、恢复和最终失败通过 JSON 事件明确通知，动态 schema 描述事件与关联错误码。
- 调试 Session 建立时真正暂停目标，再返回 Halted 状态；初始化失败会清理已取得的后端资源。
- JSON/REPL 启动观测先注册再运行，`--continue` 启动实际采样，`--halt-on-start` 优先。
- JSON 整数转换不再静默截断，已提供的可选参数会校验类型；加强动态 schema 一致性测试。
- 默认 probe-rs 单元测试不再尝试连接真实探针；断点设置失败恢复 ID 与既有地址映射。
- 采样间隔、恢复等待和锁等待响应停止；线程清理通过 join 完成，正在进行的同步读操作需先返回。
- 错误分类不再将 `communication`/`command` 误判为串口错误；失败的烧录不会自动切换后端再次写入。
- probe-rs Semihosting 空实现明确不可用；停机查询中的真实通信失败更新连接状态。
- package/lock 版本对齐为 1.0.2；CI/Release 读取提交的锁文件，取消无条件重解依赖。

### Validation boundary

本版本的[离线实现验证记录](docs/v1.0.2-validation.md)包含 144 个单元测试和 8 个进程集成测试，以及格式、Clippy、schema、Release 构建和版本检查。尚未声明真实 MCU/OpenOCD 成功路径或多平台运行测试全部通过；完整硬件重连后的 RTT、断点及采样连续性仍需独立验证。
