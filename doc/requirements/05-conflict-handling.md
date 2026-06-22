# 冲突处理

当目标 agent skill 路径已经存在时，就会发生冲突。

示例：

```text
source:
  <shared-skills-root>/skills/design-clarifier

target:
  <agent-skills-dir>/design-clarifier
```

如果目标路径已经存在，Skills Manager 不能静默覆盖。

### 冲突类型

目标路径可能是：

- 已存在的 symlink
- 已存在的普通目录
- 已存在的普通文件
- agent 提供的内置 skill
- 用户维护的本地副本
- 失效的 symlink

每一种情况都应该被清晰报告。

### 默认行为

默认行为应该保守：

```text
检测冲突 -> 报告冲突 -> 停止或跳过
```

MVP 中，冲突处理只支持跳过。Skills Manager 不提供备份后链接、接管已有目录、
替换已有 symlink 等自动处理动作。发生冲突时，工具应该保留现有目标路径不变，并向用户展示
冲突详情和需要用户手动处理的路径。

### 支持的冲突处理动作

MVP 只支持以下动作：

- `skip`：保留现有目标路径，不做修改，并提示用户手动处理冲突

### 不自动合并目录

symlink 不会合并两个 skill 目录。一个目标 skill 路径应该只指向一个被选中的源目录。

如果两个来源都定义了同名 skill 目录：

```text
source-a/skills/design-clarifier
source-b/skills/design-clarifier
```

Skills Manager 应该要求用户显式选择一个来源，而不是自动尝试合并目录。

目录合并需要 overlay 或 union filesystem，这不属于当前场景的优先方案。
