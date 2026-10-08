# DeepNote Common — Source Authority

Source Authority 是通用框架，不是固定领域排序。

## Levels

### PRIMARY_OFFICIAL

标准组织、法律原文、架构规范、官方 protocol specification、原始论文/数据。

### VENDOR_OFFICIAL

芯片厂商 Reference Manual / Datasheet / Errata / Application Note、官方产品文档。

### REFERENCE_IMPLEMENTATION

官方或事实标准实现、维护者源代码、官方 example。

### SECONDARY_TECHNICAL

高质量工程文章、书籍、独立评测、研究综述。

### COMMUNITY

论坛、StackOverflow、Reddit、个人博客、未独立验证的经验。

### PERSONAL_ANALYSIS

基于已有 Evidence 的推导，不应伪装为来源事实。

## Selection Rules

- 规范性问题优先规范/官方原文。
- 实现行为可查源码，但源码行为不自动等于标准要求。
- 厂商特定问题优先 RM / Errata / AN，再看 SDK。
- 性能收益优先完整实验条件和独立复现。
- 社区资料适合发现线索，不适合作为高风险结论的唯一证据。