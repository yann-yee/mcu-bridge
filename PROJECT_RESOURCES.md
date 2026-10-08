# Project Resource Registry

## Purpose

登记本项目可使用的长期个人知识和可复用资产。当前行为事实以项目代码、配置、测试为准；本文件不是依赖声明，也不把外部资源视为项目事实。

## Resources

**UNCONFIGURED — USER_CONFIRMED**：2026-10-08 用户选择“暂不登记，标记资源未配置”。没有已授权、已验证可访问的个人资源；不虚构知识库、代码库或连接器位置。这不阻止项目内低风险工作，后续 setup 不重复询问，除非用户提出新增/变更资源。

相邻 vibecoding/template 仅用于本次用户请求的 Core 对照，不据此登记为通用资产源；其他相邻项目也不自动获得读取、复制或修改授权。

新增每个资源时记录：

| Metadata | 内容 |
|---|---|
| Logical name / Locator Key | 稳定逻辑名，匹配本机路径映射 |
| Type | KNOWLEDGE_BASE / CODE_LIBRARY / MODEL_LIBRARY / TEMPLATE_LIBRARY / DATA_LIBRARY / OTHER |
| Domains / Purpose | 领域及为何与本项目相关 |
| Preferred Use | 哪些决策应优先查询 |
| Access | READ_ONLY / READ_WRITE / COPY_ALLOWED / REFERENCE_ONLY；读、复制、写分别声明 |
| Required | yes / no |
| Trust / Notes | 来源、时效、版本、许可、验证状态 |

## Routing Policy

项目事实 → 相关已登记个人资源 → 必要的权威外部证据 → 新研究/实现。
资源过时、不适用、不可访问时记录原因并使用下一来源；不可把缺失资源变成无限等待。

默认知识库只读参考；代码库可读取不等于可复制或修改。READ_WRITE 仅用于用户明确要求维护该资产的任务。复用前核实适用性、许可证、版本和验证状态。

## Local Path Mapping

机器路径保存到 `.agents/resources.local.md`，该文件被 .gitignore 排除。当前映射明确标记 UNAVAILABLE（未配置）。不要把机器绝对路径提交到本登记；用户明确选择共享的稳定相对路径可另行登记。
