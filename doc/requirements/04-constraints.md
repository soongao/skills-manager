# 核心约束

### 平台与能力检测

Skills Manager 启动时应该自动检测当前机器环境，并把检测结果展示给用户和 CLI。

至少需要检测：

- 操作系统类型：macOS、Linux、Windows
- CPU 架构
- 用户配置目录 `~/.skills-manager` 是否存在且可读写
- 当前用户是否有创建 symlink 的能力
- `ssh` 是否可用
- `rsync` 是否可用
- 远程同步所需依赖是否可用
- Codex、Claude Code、OpenCode 是否已安装
- 各 agent 的 skill 目录是否存在
- 各 agent 的 hook 配置文件是否存在且可解析
- 当前 agent hook 是否能早于 skill discovery 执行
- 远程机器上的 Skills Manager CLI 是否存在且版本兼容
- 本地 Desktop、CLI、远程 CLI 和配置 schema 主版本是否兼容

检测结果应该区分：

- `可用`：功能可以直接使用
- `缺少依赖`：需要安装工具或运行时，例如 `ssh` 或 `rsync`
- `权限不足`：当前用户没有执行该操作的权限，例如 Windows symlink 权限不足
- `路径不存在`：目标目录或配置文件不存在
- `无法确认`：工具无法安全判断，不应自动写入或执行破坏性操作

桌面工具和 CLI 都应该使用同一套能力检测结果。不可用能力不应该导致整个应用不可用，
而应该在具体功能上显示禁用原因。

### 跨平台路径与链接语义

Skills Manager 必须使用平台对应的路径规则，不应把类 Unix 路径规则直接套用到 Windows。

在 macOS 和 Linux 上，默认使用 symlink 管理 agent skill 引用。

在 Windows 上，优先使用目录 symlink；如果当前用户没有 symlink 权限，工具应报告权限问题，
并提示需要启用开发者模式或以具备权限的方式运行。MVP 不应静默退化为复制目录，因为复制会破坏
“一份 skill，多 agent 复用”的核心语义。

所有配置中的路径都应保存为可明确解释的形式。用户配置目录中可以保存平台相关路径；
共享源目录中的仓库元信息不得保存机器相关绝对路径。

### Skill 目录作为不透明包

Skills Manager 不需要理解 skill 目录内部内容。对工具来说，一个 skill 就是
`<shared-skills-root>/skills/<skillId>` 下的一个目录。

工具不解析、不校验、不改写该目录内部文件，也不要求目录内必须存在特定文件，例如
`SKILL.md`。是否能被某个 agent 正确识别和使用，由该 agent 自己负责。

因此，Skills Manager 的管理边界只包括：

- 发现共享源目录下的 skill 目录
- 记录 skillId 和路径
- 管理该 skill 对各 agent 的启用状态
- 创建、删除或修复指向该 skill 目录的 symlink

工具不负责根据 skill 内容生成 agent 专属适配层，也不负责合并、转换或校验 skill 内容。

### Agent 目录只是引用层

`$CODEX_HOME/skills`、`~/.agents/skills`、`~/.claude/skills` 和 `~/.config/opencode/skills`
这类 agent 专属目录应该被视为引用层，
而不是源内容目录。

事实来源应该始终是：

```text
<shared-skills-root>/skills
```

这样可以避免：

- 同一个 skill 存在多个独立副本
- 为某个 agent 修改后，其他 agent 没有同步
- 本地版本和远程版本发生漂移
- 不清楚应该修改哪一份副本
- agent 清理缓存或升级时误删自定义 skill

### Hook 必须早于 Skill 发现

如果链接动作由 agent hook 执行，hook 必须在 agent 扫描或发现 skills 之前运行。
否则本次 agent 运行可能读不到新启用的 skill，只能在下一次启动时生效。

如果某个 agent 不支持足够早的 hook 时机，Skills Manager 仍应支持由桌面工具手动触发
reconcile，或者要求用户在启动该 agent 前先应用变更。

自动安装 hook 前，Skills Manager 应确认该 agent 的 hook 时机早于 skill discovery。
如果无法确认，则不应自动安装该 hook。

对 MVP 已调研的 Codex、Claude Code、OpenCode，当前结论是都不能默认假设 hook 早于
skill discovery。因此，hook 自动安装应作为“已验证后启用”的能力，而不是 MVP 的主生效路径。

### Hook 安装必须安全可回滚

Skills Manager 自动修改 agent 配置时，必须保证用户可以恢复到修改前状态。

要求：

- 每次修改前创建带时间戳的备份
- 记录 Skills Manager 写入的 hook 标识
- 支持卸载 Skills Manager hook
- 卸载时只删除 Skills Manager 写入的 hook，不影响用户其他配置
- 配置文件格式无法解析或存在未知结构时，不进行写入

### Hook 在 Agent 所在环境中运行

hook 应该在 agent 实际运行的机器和用户环境中执行。这样它创建的 symlink 才能指向
agent 可见的路径。

本地 agent 场景：

```text
<codex-skills-dir>/<skillId> -> <shared-skills-root>/skills/<skillId>
```

远程 agent 场景：

```text
<codex-skills-dir>/<skillId> -> ~/.skills-manager/cache/<repo-id>/skills/<skillId>
```

桌面工具不应该假设本地路径在远程机器上也存在。远程 hook 需要读取远程可见的配置，
并使用远程机器上的缓存路径。

### 远程访问依赖同步缓存状态

远程机器上的 symlink 不能直接指向本地机器路径。MVP 中，agent 所在机器必须先拥有一个本机可见的
skills 缓存目录，例如：

```text
~/.skills-manager/cache/<repo-id>/skills
```

如果同步缓存不可用、过期或同步失败，agent 可能读不到最新 skills。这意味着：

- 本地机器必须在线
- 本机必须能通过 SSH 访问远程机器
- `rsync` 或等价同步能力必须可用
- 网络故障可能导致远程缓存暂时落后于事实来源
- 远程 agent 可以继续读取上一次成功同步的缓存

Skills Manager 应该显式暴露这个依赖，并负责远程同步、缓存健康检查、最后同步时间、
缓存是否过期和错误展示。当同步失败时，工具不应该删除既有远程缓存，也不应该继续对远程 agent
执行会指向不存在缓存的链接变更。

缓存过期或同步失败不应阻止 agent 启动。hook 和 CLI 应记录 `cache stale` 或 `sync failed` 状态，
agent 继续使用已有缓存或已有链接启动，桌面工具负责提示用户处理。

### SSH 凭据不由工具托管

Skills Manager 不托管 SSH 凭据。

要求：

- 不保存 SSH 密码
- 不保存 SSH 私钥
- 不保存 SSH key passphrase
- 不实现自有凭据加密存储
- 只调用系统已有的 SSH、rsync 或等价同步能力
- 认证失败时展示错误和排查提示，由用户修复系统 SSH 配置

因此，远程功能依赖用户已经可以从本机通过系统 SSH 成功连接目标机器。
