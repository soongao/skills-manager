# 实现契约

### JSON 字段命名

MVP 的配置、状态、CLI JSON 输出和运行记录统一使用以下命名规则：

- JSON 字段名使用 lower camelCase，例如 `activeSourceProfileId`、`enabledSkillIds`
- 稳定枚举值使用 lower kebab-case，例如 `push-local-to-remote`、`claude-code`
- `agentId` 固定为 `codex`、`claude-code`、`opencode`
- `sourceProfileId`、`environmentId`、`skillId` 使用字符串保存；工具推荐 lower kebab-case，但不强制改写用户已有目录名
- 时间使用 RFC 3339 UTC 字符串，例如 `2026-06-22T10:00:00Z`
- 配置文件可以保存 `~`、环境变量或相对路径；执行前必须解析为绝对路径，并在 `state.json` 或运行记录中保存 resolved path
- 布尔字段使用正向语义，例如 `enabled`、`managed`、`autoSync`、`deleteExtraneous`

路径字段固定命名：

- `sourceRoot`：本机 source profile 的事实来源目录
- `remoteSourceRoot`：远程 source profile 的事实来源目录
- `localCacheRoot`：远程源同步到本机后的缓存目录
- `remoteCacheRoot`：本机源同步到远程后的缓存目录
- `skillsDir`：某个 environment 中某个 agent 的 skill 目录

缓存 marker 文件固定为 `<cacheRoot>/.skills-manager-cache.json`。推荐格式：

```json
{
  "schemaVersion": 1,
  "managedBy": "skills-manager",
  "repoId": "personal-skills",
  "sourceProfileId": "local-personal",
  "createdAt": "2026-06-22T10:00:00Z"
}
```

### CLI JSON 输出

所有可被桌面工具、hook 或自动化调用的 CLI 命令都必须支持 `--json`，并返回统一结构：

```json
{
  "schemaVersion": 1,
  "ok": true,
  "status": "success",
  "command": "reconcile",
  "runId": "20260622-100000-abcdef",
  "startedAt": "2026-06-22T10:00:00Z",
  "endedAt": "2026-06-22T10:00:01Z",
  "summary": {
    "applied": 1,
    "skipped": 0,
    "conflicts": 0,
    "errors": 0
  },
  "actions": [
    {
      "type": "create-symlink",
      "status": "applied",
      "environmentId": "local",
      "agentId": "claude-code",
      "skillId": "design-clarifier",
      "sourcePath": "/path/to/shared-skills/skills/design-clarifier",
      "targetPath": "/Users/alice/.claude/skills/design-clarifier"
    }
  ],
  "warnings": [],
  "errors": []
}
```

`status` 取值：

- `success`：命令完成且没有错误
- `partial`：命令完成，但存在跳过项、冲突或非阻塞错误
- `failed`：命令未能完成核心动作

`actions[].status` 取值：

- `planned`
- `applied`
- `skipped`
- `conflict`
- `failed`

### 错误码

错误码必须稳定，供 CLI、桌面工具、hook 和日志共同使用。错误码使用大写 snake case。

MVP 错误码至少包括：

| 错误码 | 含义 |
| --- | --- |
| `CONFIG_INVALID` | 配置文件不可解析或字段不合法 |
| `CONFIG_UNSUPPORTED_VERSION` | 配置 schema 主版本不兼容 |
| `SOURCE_NOT_FOUND` | active source profile 的源目录不存在 |
| `SOURCE_INVALID_LAYOUT` | 源目录缺少可用的 `skills/` 结构 |
| `CACHE_MARKER_MISSING` | 缓存目录缺少 `.skills-manager-cache.json` |
| `CACHE_MARKER_MISMATCH` | 缓存 marker 与当前 repo 或 source profile 不匹配 |
| `SYNC_SSH_UNAVAILABLE` | 本机无法调用 `ssh` |
| `SYNC_RSYNC_UNAVAILABLE` | 本机无法调用 `rsync` |
| `SYNC_FAILED` | 同步命令执行失败 |
| `AGENT_NOT_DETECTED` | 未检测到指定 agent |
| `AGENT_SKILLS_DIR_INVALID` | agent skill 目录不可用或无法创建 |
| `RECONCILE_TARGET_CONFLICT` | 目标路径存在且不能安全管理 |
| `RECONCILE_SOURCE_MISSING` | 期望链接的源目录不存在 |
| `RECONCILE_PERMISSION_DENIED` | 创建、删除或读取路径时权限不足 |
| `HOOK_UNSUPPORTED` | 当前 agent 不支持自动安装 hook |
| `HOOK_VERSION_UNVERIFIED` | 当前 agent 版本的 hook 时机未验证 |
| `HOOK_CONFIG_CONFLICT` | hook 配置文件存在无法安全修改的结构 |
| `REMOTE_CLI_MISSING` | 远程机器缺少 Skills Manager CLI |
| `VERSION_MISMATCH` | Desktop、CLI、远程 CLI 或 schema 主版本不兼容 |

错误对象统一格式：

```json
{
  "code": "RECONCILE_TARGET_CONFLICT",
  "message": "Target path already exists and is not a managed symlink.",
  "environmentId": "local",
  "agentId": "claude-code",
  "skillId": "design-clarifier",
  "path": "/Users/alice/.claude/skills/design-clarifier"
}
```

### 桌面 UI 信息结构

MVP 桌面 UI 按信息架构划分，不强制具体视觉布局：

- Source：展示 active source profile、source 类型、本机路径或远程路径、repo 元信息、扫描结果
- Skills：展示 `skillId` 列表、源路径、每个 environment/agent 的启用状态和实际状态
- Environments：展示本机和远程 environment，包含 host、agent 检测状态、agent `skillsDir`、managed 开关
- Sync：展示远程同步方向、cache 路径、marker 状态、最近同步时间、同步结果、手动同步按钮和 `autoSync`
- Hooks：展示每个 environment/agent 的 hook 支持状态、安装状态、版本验证状态和配置冲突
- Issues：集中展示冲突、缓存过期、同步失败、权限不足、版本不兼容等需要用户处理的问题
- Settings：配置用户目录、日志位置、默认路径检测结果和版本信息

UI 中的主要状态文案应直接映射到状态模型和错误码，避免桌面工具维护另一套隐式状态。

### 日志和运行记录格式

日志文件使用 JSON Lines，路径固定为 `~/.skills-manager/logs/skills-manager.log`。
每一行是一条独立 JSON 事件：

```json
{
  "timestamp": "2026-06-22T10:00:00Z",
  "level": "info",
  "runId": "20260622-100000-abcdef",
  "component": "cli",
  "event": "reconcile.action",
  "sourceProfileId": "local-personal",
  "environmentId": "local",
  "agentId": "claude-code",
  "skillId": "design-clarifier",
  "message": "Created managed symlink.",
  "details": {
    "targetPath": "/Users/alice/.claude/skills/design-clarifier"
  }
}
```

`level` 取值为 `debug`、`info`、`warn`、`error`。`component` 取值为
`desktop`、`cli`、`hook`、`sync`、`reconcile`、`hook-installer`。

每次 CLI、hook、sync 或桌面触发的写入操作，都应在 `~/.skills-manager/runs/`
写入一份运行记录，文件名格式为：

```text
<startedAtCompact>-<runId>-<command>.json
```

运行记录保存完整 CLI JSON 输出、配置版本、仓库版本、执行环境、动作列表、错误和警告。
日志和运行记录都不得写入 SSH 密码、私钥、passphrase、完整环境变量或其他敏感凭据。
