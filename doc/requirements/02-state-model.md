# 状态模型

### 配置分层

Skills Manager 的配置采用“两者结合”的方式：

- 共享源目录保存仓库元信息
- 用户配置目录保存机器相关状态

共享源目录中的元信息描述这个 skill 仓库自身，例如仓库名称、版本和 skills 清单等。
这类信息可以随共享源目录一起移动或同步。

用户配置目录固定为 `~/.skills-manager`。其中的状态描述当前用户和当前机器如何使用这个仓库，例如共享源目录路径、
agent skill 目录、每个 agent 启用的 skills、远程主机配置、远程源目录、远程缓存目录和本机冲突决策。
这类信息通常不适合写入共享源目录，因为不同机器和用户的路径、权限、agent 安装位置可能不同。

示例布局：

```text
<shared-skills-root>/
  .skills-manager/
    repository.json
  skills/
    <skillId>/
      ...

~/.skills-manager/
  config.json
  state.json
  logs/
    skills-manager.log
  runs/
```

配置文件和 CLI 都需要记录版本信息，用于兼容性检查。MVP 采用主版本兼容策略：

- Desktop、CLI、远程 CLI 的主版本必须一致，才允许执行写入类操作
- 小版本或补丁版本不一致时，可以继续运行，但应展示版本提示
- `repository.json`、`config.json`、`state.json` 都应包含 schema/version 字段
- 如果配置文件主版本高于当前程序支持版本，程序不应写入该文件
- 如果配置文件主版本低于当前程序支持版本，程序可以提示迁移或在用户确认后升级

写入类操作包括：更新配置、创建或删除 symlink、安装 hook、修改远程同步配置、写入
`state.json` 等。

日志和运行记录也统一放在 `~/.skills-manager` 中。hook、CLI 和桌面工具应记录关键操作、
失败原因、冲突项和远程同步错误，便于用户排查。

推荐布局：

```text
~/.skills-manager/
  logs/
    skills-manager.log
  runs/
    <timestamp>-<command>.json
```

日志不应包含 SSH 密码、私钥、passphrase 或其他敏感凭据。

### 仓库元信息

仓库元信息放在共享源目录中，作为该 skill 仓库的说明和索引。

Skills Manager 应该负责初始化共享源目录。用户选择一个目录后，如果该目录缺少
`.skills-manager/repository.json`，工具仍然可以先扫描 `skills/` 目录并展示可发现的 skills，
同时提示用户初始化仓库元信息。

初始化时，工具可以创建：

```text
<shared-skills-root>/
  .skills-manager/
    repository.json
  skills/
```

如果 `skills/` 目录不存在，工具可以在用户确认后创建它。

`repository.json` 是仓库元信息的推荐持久化文件，但不是发现 skills 的唯一前置条件。
没有该文件时，工具不应直接判定共享源目录不可用。

MVP 只扫描 `<shared-skills-root>/skills` 下的一级子目录作为 skills，不扫描共享源目录根目录下的其他目录。

Skills Manager 可以自动更新 `<shared-skills-root>/.skills-manager/repository.json`，用于维护仓库元信息和
skills 清单。该写入边界仅限 `.skills-manager/` 元信息目录，不能修改
`<shared-skills-root>/skills/<skillId>/` 内部内容。

示例：

```json
{
  "name": "personal-skills",
  "version": 1,
  "schemaVersion": 1,
  "skills": [
    {
      "skillId": "design-clarifier",
      "path": "skills/design-clarifier"
    }
  ]
}
```

仓库元信息不应该保存本机路径、远程源目录、远程缓存目录、用户私有启用状态或 agent 本地目录。

### 期望状态

Skills Manager 应该把用户选择保存为明确的期望状态。该状态存放在用户配置目录中，
描述当前 active source profile 是什么、每个 environment 如何访问这些 skills，
以及当前用户希望哪些 skills 对哪些 environment 中的 agent 启用。

示例：

```json
{
  "schemaVersion": 1,
  "activeSourceProfileId": "local-personal",
  "sourceProfiles": {
    "local-personal": {
      "kind": "local",
      "sourceRoot": "/path/to/shared-skills"
    },
    "team-skills": {
      "kind": "remote",
      "host": "skills-host.example.com",
      "user": "alice",
      "remoteSourceRoot": "/srv/team-skills",
      "localCacheRoot": "~/.skills-manager/cache/team-skills",
      "autoSync": false,
      "deleteExtraneous": true
    }
  },
  "environments": {
    "local": {
      "kind": "local",
      "agents": {
        "codex": {
          "managed": true,
          "skillsDir": "$CODEX_HOME/skills",
          "enabledSkillIds": ["design-clarifier", "api-test"]
        },
        "claude-code": {
          "managed": true,
          "skillsDir": "~/.claude/skills",
          "enabledSkillIds": ["design-clarifier"]
        },
        "opencode": {
          "managed": false,
          "skillsDir": "~/.config/opencode/skills",
          "enabledSkillIds": []
        }
      }
    },
    "devbox": {
      "kind": "remote",
      "host": "devbox.example.com",
      "user": "alice",
      "enabled": true,
      "direction": "push-local-to-remote",
      "remoteCacheRoot": "~/.skills-manager/cache/personal-skills",
      "autoSync": false,
      "deleteExtraneous": true,
      "agents": {
        "claude-code": {
          "managed": true,
          "skillsDir": "~/.claude/skills",
          "enabledSkillIds": ["design-clarifier"]
        }
      }
    }
  }
}
```

在这个模型里，source profile 决定 skills 从哪里来，environment 决定这些 skills 应用到哪台机器的哪些 agent。
“启用”表示用户希望某个 environment 中的某个 agent 能使用这个 skill；实际文件系统中的 symlink 是否已经存在，
或者 OpenCode 原生 skill paths 是否已经配置，是实际状态。

### 实际状态

实际状态来自文件系统扫描。对每个 skill 和 agent，可能出现以下状态：

- `未启用`：期望状态中未启用，目标 agent 目录也没有对应链接
- `待应用`：期望状态中已启用，但目标 agent 目录还没有对应链接
- `已启用`：目标 agent 目录存在 symlink，并且指向正确的共享 skill
- `部分启用`：一个 skill 对部分 agent 已启用，对其他 agent 未启用或待应用
- `冲突`：目标路径已存在，但不是可安全管理的正确 symlink
- `失效`：目标路径是 symlink，但源目录不存在或不可访问
- `缓存过期`：agent 指向的缓存目录存在，但最近一次同步失败、超时或落后于事实来源
- `同步失败`：remote profile 的最近一次同步失败，需要用户查看错误并重试

桌面工具应该展示期望状态和实际状态的差异，让用户知道当前配置是否已经生效。

### Reconcile

reconcile 是把实际状态收敛到期望状态的过程。它可以由桌面工具主动触发，也可以由
agent hook 在 agent 启动前通过 Skills Manager CLI 触发。

reconcile 应该只对明确由 Skills Manager 管理的链接执行自动操作：

- 期望启用且目标不存在时，创建 symlink
- 期望禁用且目标是受管理 symlink 时，删除 symlink
- 目标路径已存在且不是受管理 symlink 时，跳过并报告冲突
- 目标路径是错误 symlink、普通文件或普通目录时，跳过并报告冲突
- 受管理 symlink 的源目录不存在时，标记为失效，不自动删除

禁用 skill 时，只能删除由 Skills Manager 创建或已登记为受管理的 symlink。如果目标路径是普通目录、
普通文件、未知来源 symlink，或无法确认是否由 Skills Manager 管理，则不能删除，只能报告冲突。

当共享源目录中的 skill 被删除或暂时不可访问时，已启用的受管理 symlink 会变成失效链接。
MVP 中，reconcile 不自动删除这些失效链接。桌面工具应展示失效状态和对应路径，并允许用户在确认后
清理这些受管理 symlink。

受管理 symlink 的判断规则：

- `state.json` 中登记过的 symlink 视为受管理
- 未登记在 `state.json` 中的 symlink 即使指向正确共享 skill，也不自动纳入管理
- hook 和默认 CLI reconcile 不应处理未知 symlink，应该跳过并报告冲突

hook 中的 reconcile 应该是非交互式的。遇到需要用户决策的冲突时，hook 应该跳过该项、
记录状态，并让桌面工具展示给用户处理，避免 agent 启动过程被交互阻塞。

CLI 中的 reconcile 默认也必须是非交互式的，适合被 hook 调用。需要用户确认的操作只能在
桌面工具中完成，或者通过显式的 CLI 参数启用。

hook 调用 CLI 时应使用 `--json`，并根据结构化结果记录告警或提示用户打开桌面工具处理。
MVP 中，hook 调用 `skills-manager reconcile` 失败不应阻止 agent 启动。失败只应写入状态、
日志或告警，agent 继续按原流程启动。
