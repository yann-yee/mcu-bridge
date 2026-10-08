# Project Resource Registry Template

本模板用于生成项目根目录 `PROJECT_RESOURCES.md`。

它登记“项目可以使用哪些长期个人资源”，不是依赖锁文件，也不是把资源内容复制进当前项目。

---

# PROJECT_RESOURCES.md

## Purpose

This project may use the following personal knowledge and reusable asset sources.

Project code/config/tests remain the source of truth for current implementation behavior.

## Resources

### <logical-name>

- **Type:** `KNOWLEDGE_BASE / CODE_LIBRARY / MODEL_LIBRARY / TEMPLATE_LIBRARY / DATA_LIBRARY / OTHER`
- **Domains:** embedded / ai / frontend / backend / legal / ...
- **Purpose:** 为什么这个项目可能需要它
- **Locator Key:** 用于匹配本地路径映射，例如 `personal-knowledge`
- **Access:** `READ_ONLY / READ_WRITE / COPY_ALLOWED / REFERENCE_ONLY`
- **Required:** yes / no
- **Preferred Use:** 什么时候优先查询
- **Trust / Notes:** 可信度、版本、许可证或其它限制

## Routing Policy

Recommended default:

```text
Current project evidence
        ↓
Registered personal resource when relevant
        ↓
Authoritative external evidence if needed
        ↓
New research / new implementation
```

Exceptions are allowed when the personal resource is stale, incompatible, lower authority, unavailable or clearly irrelevant.

## Local Path Mapping

Machine-specific paths SHOULD live in:

`.agents/resources.local.md`

Example:

```markdown
# Local Resource Paths

- personal-knowledge: D:/Knowledge
- embed-lib: D:/code/embed-lib
- ai-lib: D:/code/ai-lib
```

Add the local mapping file to `.gitignore` unless the user intentionally wants it versioned.

## Example

### personal-knowledge

- **Type:** `KNOWLEDGE_BASE`
- **Domains:** embedded, ai, software, legal
- **Purpose:** personal Obsidian knowledge and source notes
- **Locator Key:** `personal-knowledge`
- **Access:** `READ_ONLY`
- **Required:** no
- **Preferred Use:** check before broad external research; verify critical claims when freshness matters
- **Trust / Notes:** note status/source metadata may indicate confidence

### embed-lib

- **Type:** `CODE_LIBRARY`
- **Domains:** embedded
- **Purpose:** reusable embedded components and selected upstream libraries
- **Locator Key:** `embed-lib`
- **Access:** `COPY_ALLOWED`
- **Required:** no
- **Preferred Use:** inspect CATALOG/README/SOURCE before implementing generic embedded capability
- **Trust / Notes:** availability does not imply suitability; check maturity/verification/license
