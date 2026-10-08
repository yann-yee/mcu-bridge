# Engineering Notes

从旧 AGENTS 迁出的按需细节；长期约束以根 AGENTS.md 为准。以下历史经验不能替代当前锁定依赖源码或实际测试证据。

## probe-rs API and ownership

- 先核对 Cargo.lock 的 probe-rs 版本，再仅检查所需 API 的源码/文档签名。不要广泛下钻 SDK；编译器用于核实已选调用路径。旧版笔记中的 re-export/trait 归属不可直接当成当前事实。
- 当前锁定 0.31.0。Session/Permissions 等类型的公开导出可能不同于内部模块路径；内存方法须核对 MemoryInterface。具体 halt/run/step 方法归属也应现场确认。
- 方法同时改断点 ID/map 并借出 Core 时，先准备 self 状态，再借子对象进行硬件操作，失败回滚状态；勿先借 Core 再改 self。
- 同时借 rtt/session 不同字段时，整体借 self 的 get_core() 可能扩大借用；显式字段访问有助于 Rust 的字段级借用追踪。避免未经核实就更改封装 API。
- probe-rs target 名须精确。旧经验示例 STM32F411RE 可识别而 STM32F411 不能据此假定可识别；用户 --chip 原值透传，模板只校验/提供架构参数。

## OpenOCD TCL parsing

- read_memory 返回 `{0x12345678 0x9abcdef0 ...}` 时去掉外层花括号，按 whitespace 解析 word；支持 0x/0X 前缀，不按行假定一个 word。
- reg 可能输出 `(0) r0 (/32): 0x00000000 (dirty)`：跳过非 `(` 行，按冒号分 name/value，值取第一个 whitespace token，忽略 dirty 后缀；无法解析应跳过，不 panic。
- wait_halt 成功可能是空响应，不能要求成功关键词；需检查 timeout。halt/step/resume/bp/rbp 等须识别 error/failed 响应。解析行为必须用响应样本证明，不依赖字符串猜测。
- 正常退出和 Ctrl+C 均需检查进程残留。Drop 不足以保证所有 OS 强制终止情形；不要把 cleanup 设计要求表述为已验证的绝对保证。

## Build and CI pitfalls

- 新环境先 cargo fetch 再编译，避免把下载耗时当编译挂起。
- Windows 独立 MSI 历史上可能缺 clippy-driver；先验证 driver。缺失时 fmt/check 和全 src 同模式搜索只作诊断，不证明 Clippy 通过、不许可 suppression。
- Cargo.lock 是二进制项目应跟踪的产物，CI 缓存依赖它。当前 CI/release 会重新 generate-lockfile；尚未证明 CI 使用提交时的精确依赖集合。
- 跨 job 发布通过 upload/download-artifact v4，按 target triple 唯一命名后集中 release。当前实现见 workflows，不在此复制 YAML。

## Reference authority

- 当前 manifest/lock/source/CI 是实现事实；README 是入口和产品意图，可能有过时示例。
- 旧 AGENTS 引用的 context.md 原本缺失，现已建立；嵌入式调试软件.md 当前仓库未找到，不能要求未来 Agent 假装已读。
- 权威依赖资料：docs.rs/probe-rs、docs.rs/clap；重要 API 结论需匹配 Cargo.lock 版本。
