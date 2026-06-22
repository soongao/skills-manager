# 目标与能力范围

## 背景

不同 agent 的 skill 安装和发现路径并不一致。比如 Codex 和 Claude Code
可能分别维护自己的 skill 目录。如果同一个 skill 被复制到多个 agent
专属目录里，这些副本会随着时间产生漂移，也会很难判断哪一份才是事实来源。

Skills Manager 的目标是管理一个可配置的共享 skill 源目录，让其中的 skills
可以被多个 agent 复用，并且在需要时，也能在本机和远程机器之间同步使用。

## 目标

### 1. 统一 Skill 源目录

MVP 同一时间只允许一个 active source profile。这个 source profile 决定当前 skills 的事实来源，
可以是本机目录，也可以是远程目录同步到本机缓存后的目录。
所有 environment 在 MVP 中都使用同一个 active source profile；environment 不单独选择 source profile。

不管 source profile 来自哪里，源目录中的 skill 结构固定为
`<shared-skills-root>/skills/<skillId>`：

```text
<shared-skills-root>/
  skills/
    <skillId>/
      ...
```

agent 专属 skill 目录不应该保存共享 skill 的独立副本，而应该通过链接引用共享源目录。

推荐目录结构：

```text
<codex-skills-dir>/design-clarifier
  -> <shared-skills-root>/skills/design-clarifier

~/.claude/skills/design-clarifier
  -> <shared-skills-root>/skills/design-clarifier

~/.config/opencode/skills/design-clarifier
  -> <shared-skills-root>/skills/design-clarifier
```

其中 `<codex-skills-dir>` 不能硬编码为单一路径。Codex 需要按当前安装形态检测可用目录：
官方 manual 记录的用户 skill 目录是 `$HOME/.agents/skills`，但当前 Codex 自带的 skill installer
会安装到 `$CODEX_HOME/skills`，在 `CODEX_HOME` 未设置时通常是 `~/.codex/skills`。
Skills Manager 应把两者都作为 Codex 候选目录检测，并让用户选择最终管理哪个目录。

链接应该按单个 skill 粒度创建，而不是把整个 agent skill 目录替换成共享目录。
这样可以保留 agent 自己的内置 skills、缓存文件、插件或私有 skills。

MVP 第一批内置支持以下 agent：

- Codex
- Claude Code
- OpenCode

其他 agent 可以作为后续扩展，不进入 MVP 的内置支持范围。

MVP 支持的桌面运行平台包括：

- macOS
- Linux
- Windows

不同平台的 symlink、SSH/rsync、用户配置目录和 agent 配置路径存在差异。Skills Manager
应该在运行时自动检测当前机器能力，并根据检测结果决定哪些功能可用。

### 2. 双向远程 Skills 使用

远程管理属于 MVP 范围。MVP 需要支持远程主机配置、同步状态检测、远程 CLI 检测、
远程 hook 安装状态检测，以及远程 agent 的 reconcile 状态展示。

MVP 的远程访问主路径采用 `sync-cache` 模式，而不是 `sshfs` 挂载模式。也就是说，
事实来源目录先同步到 agent 所在机器的缓存目录，agent 再通过 symlink 或 agent 原生配置读取该缓存目录。
这样只要求本机可以通过 SSH 访问远程机器，不要求远程机器反向访问本机，也不要求本机开启 SSH server。

MVP 支持两个方向：

- `push-local-to-remote`：本机共享 skills 给远程 agent 使用。本机是事实来源，远程机器保存只读缓存。
- `pull-remote-to-local`：本机 agent 使用远程 skills。远程目录是事实来源，本机保存只读缓存。

缓存目录不是事实来源。Skills Manager 不应把用户对缓存目录的修改反向写回源目录。
当缓存内容和源目录不一致时，以对应 profile 的 sourceRoot 为准，下次同步会覆盖缓存差异。
同步默认使用删除语义，例如 `rsync --delete`。当源目录删除某个 skill 后，缓存中的对应目录也应在下次同步时删除。

远程同步范围固定为：

- `skills/`
- `.skills-manager/repository.json`

源目录下的其他文件和目录不进入 MVP 同步范围。

由于同步默认带删除语义，缓存目录必须先由 Skills Manager 初始化并写入 marker 文件，例如
`<cacheRoot>/.skills-manager-cache.json`。只有检测到 marker 且其中的 `repoId`、`sourceProfileId`
与当前配置匹配时，才允许执行带删除语义的同步。没有 marker、marker 无法解析或 marker 不匹配时，
同步必须停止并提示用户重新初始化缓存目录，不能执行 `rsync --delete`。

远程同步默认由用户手动触发。MVP 可以提供可选的自动监听同步：当 sourceRoot 下的文件发生变化时，
经过短暂 debounce 后自动同步到缓存目录。自动监听同步默认关闭，用户可以按 remote profile 单独启用。
无论手动还是自动同步，桌面工具都应展示最近一次同步时间、同步方向、同步结果和错误信息。

MVP 中，远程 SSH 凭据复用系统 SSH 配置。Skills Manager 不保存 SSH 密码、私钥或
passphrase。工具只保存 host、user、远程源目录、远程缓存目录等非敏感配置，并依赖用户已有的
`~/.ssh/config`、SSH key 和 ssh-agent 完成认证。

`push-local-to-remote` 示例结构：

```text
本机:
<local-shared-skills-root>/skills

远程:
~/.skills-manager/cache/<repo-id>/skills

<codex-skills-dir>/design-clarifier
  -> ~/.skills-manager/cache/<repo-id>/skills/design-clarifier

~/.claude/skills/design-clarifier
  -> ~/.skills-manager/cache/<repo-id>/skills/design-clarifier

OpenCode:
  优先通过额外 skill paths 指向 ~/.skills-manager/cache/<repo-id>/skills
```

示例同步命令：

```bash
rsync -az --delete \
  <local-shared-skills-root>/skills/ \
  <user>@<remote-host>:~/.skills-manager/cache/<repo-id>/skills/
```

`pull-remote-to-local` 示例结构：

```text
远程:
<remote-shared-skills-root>/skills

本机:
~/.skills-manager/cache/<remote-repo-id>/skills

本机 agent:
<agent-skills-dir>/design-clarifier
  -> ~/.skills-manager/cache/<remote-repo-id>/skills/design-clarifier
```

示例同步命令：

```bash
rsync -az --delete \
  <user>@<remote-host>:<remote-shared-skills-root>/skills/ \
  ~/.skills-manager/cache/<remote-repo-id>/skills/
```

`sshfs` 可以作为后续高级模式，但不作为 MVP 的主路径。

### 3. 桌面工具管理 Skill 启用状态

Skills Manager 应该首先是一个桌面管理工具。用户通过它完成：

- 选择共享 skill 源目录
- 扫描源目录中的 skills
- 查看每个 skill 的 skillId、路径和状态
- 配置不同 agent 的 skill 目录
- 选择哪些 skill 对哪些 agent 启用
- 查看冲突、失效链接和未应用变更

桌面工具管理的是“期望状态”，不要求所有链接动作都必须在桌面工具进程内完成。
链接创建、更新和删除可以由 agent 启动时的 hook 执行。MVP 中 hook 时机选择 session start。

推荐职责划分：

```text
Skills Manager 桌面工具
  管理 source profile、environment、agent 配置、skill 启用选择、冲突决策和状态展示

Agent hook / reconciler
  根据桌面工具写入的期望状态，在 agent 运行环境中创建、更新或删除 symlink
```

桌面工具也可以提供“立即应用”能力，主动执行同一套 reconcile 逻辑。这样用户不必等待
agent 下一次启动，也能手动把当前配置应用到本机。

启用状态的核心模型是 `environment × skill × agent` 矩阵。每个 environment 表示一台运行 agent
的机器或上下文，例如本机环境、某个远程 devbox 环境。每个 skill 可以分别在不同 environment
里的 Codex、Claude Code、OpenCode 启用或禁用。桌面 UI 可以提供批量操作，例如
“对当前 environment 的所有 agent 启用该 skill”、“对当前 agent 启用全部 skills”，
但底层状态仍按 environment 和 agent 分别保存 `enabledSkillIds`。

`skillId` 是 Skills Manager 的文件系统管理标识，取自 active source profile 可见的 `skills` 目录下的一级目录名。
如果 active source profile 是远程源，本机扫描的是同步后的本机缓存目录。
它不等同于 skill 内部内容声明的名称；Skills Manager 不解析 skill 内部文件，也不读取内部 name 字段。

每个 agent 还应有 agent 级别的管理开关，例如 `managed: true/false`。只有
`managed: true` 的 agent 才会被 Skills Manager 纳入管理范围，包括执行 reconcile、写入新增 skill
的默认启用状态，以及在 hook 能力已验证时安装 hook。
用户可以在桌面工具中启用或关闭某个 agent 的管理。

首次初始化时，Skills Manager 应检测 Codex、Claude Code、OpenCode 的安装和配置状态。
对检测到的 agent，默认设置为 `managed: true`，但必须在引导界面展示给用户确认，用户可以取消某个 agent
的管理。未检测到的 agent 默认不纳入管理。

如果已纳入管理的 agent skill 目录不存在，Skills Manager 可以自动创建该目录。创建空的
skills 目录属于低风险操作，不需要用户手动准备。

多数 agent 的 skill 目录可以理解为其配置根目录下的 `skills` 子目录，但不同 agent 的配置根目录可能不同。
因此，工具应为 Codex、Claude Code、OpenCode 分别维护默认路径规则，并允许用户在桌面工具中修改。
不能只根据目录名 `skills` 做全局硬编码。

如果同一个目录被多个 agent 共同识别，例如 Codex 和 OpenCode 都可能读取 `~/.agents/skills`，
Skills Manager 允许多个 agent 指向同一个 `skillsDir`。桌面工具应明确标注这些 agent 正在共享同一个目标目录，
避免用户误以为每个 agent 都有独立引用层。

当用户将某个 agent 的 `managed` 从 `true` 改为 `false` 时，Skills Manager 应停止对该 agent
执行后续 hook 安装、reconcile 和新增 skill 默认启用，但不删除该 agent 目录下已经存在的 symlink。
已有链接保持原状，由用户自行决定是否手动清理。

当 active source profile 中新增 skill 目录时，Skills Manager 应在下次扫描时发现它，并默认对所有
已启用且已配置的 environment 中的 MVP 内置 agent 启用。也就是说，新 skill 会自动写入当前已纳入管理的
Codex、Claude Code、OpenCode 的 `enabledSkillIds`，随后由对应 environment 的 reconcile 创建链接或更新
agent 原生配置。未安装、未配置或被用户关闭管理的 agent 不应被写入启用状态。

如果用户重命名 skill 目录，MVP 不自动识别重命名。旧目录按删除处理，新目录按新增 skill 处理。

### 4. 提供 Hook 可调用的 CLI 入口

MVP 应同时提供桌面 App 和命令行入口。桌面 App 负责配置管理、状态展示和用户决策；
CLI 负责让 agent hook 以非交互方式调用同一套底层能力。

agent hook 不应该各自实现读取配置、创建 symlink、删除 symlink 和处理冲突的逻辑。
它们应该调用 Skills Manager CLI：

```bash
skills-manager reconcile --agent codex
skills-manager reconcile --agent claude-code
skills-manager reconcile --agent opencode
```

这样可以保证桌面工具里的“立即应用”和 agent hook 中的自动收敛使用同一套规则。

推荐能力划分：

```text
Skills Manager Desktop
  管理配置、展示状态、处理冲突、触发立即应用

skills-manager CLI
  提供 status、reconcile、remote sync、remote status 等非交互命令

Agent hook
  在 session start 时调用 skills-manager CLI，不直接实现链接逻辑
```

CLI 默认输出面向人类阅读的文本；所有需要被桌面工具、hook 或自动化流程调用的命令，都应支持
`--json` 参数输出机器可读结果。桌面工具和 hook 应优先使用 JSON 输出，避免解析文本。

示例：

```bash
skills-manager status --json
skills-manager reconcile --agent codex --json
skills-manager remote status devbox --json
skills-manager remote sync devbox --direction push-local-to-remote --json
```

JSON 输出应包含命令是否成功、实际执行的操作、跳过项、冲突项、错误码和可展示错误信息。

远程 agent 场景下，远程机器也必须安装 Skills Manager CLI。远程 hook 在远程机器本地执行
`skills-manager reconcile --agent <agent>`，不依赖本地桌面工具通过 SSH 代替它执行链接逻辑。

本地桌面工具应该检测远程机器上的 CLI 是否存在、是否可执行、版本是否兼容。如果远程 CLI
不可用，远程 hook 安装和远程 reconcile 应标记为不可用；远程 sync-cache 同步本身仍可由本机通过
SSH/rsync 执行，但同步后无法自动在远程执行 reconcile。

MVP 中，远程机器上的 Skills Manager CLI 由用户自行安装。桌面工具只负责检测远程 CLI
状态、展示版本兼容性和提供安装指引，不通过 SSH 自动上传、安装或升级远程 CLI。

### 5. 自动安装 Agent Hook

MVP 中，Skills Manager 可以在确认具体 agent 和版本的 session start hook 足够早时，自动安装 hook，
将 `skills-manager reconcile --agent <agent>` 接入 agent 的启动流程。

需要检测 hook 能力的目标 agent 包括：

- Codex
- Claude Code
- OpenCode

自动安装 hook 不是默认假设。只有当工具能确认该 agent 的 hook 配置格式可安全修改，
并且 session start hook 执行时机早于该 agent 的 skill discovery，才允许安装。否则该 agent 的状态应显示为
`不支持自动安装` 或 `需要版本验证`，并依赖桌面工具或 CLI 的“立即应用”能力。

桌面工具应该展示每个 agent 的 hook 安装状态，例如：

- `未安装`
- `已安装`
- `需要更新`
- `配置冲突`
- `不支持自动安装`

安装 hook 时必须遵守安全边界：

- 修改 agent 配置前先备份原文件
- 安装过程必须幂等，重复安装不应产生重复 hook
- 只更新 Skills Manager 自己管理的 hook 片段
- 不删除或改写用户已有的其他 hook 配置
- 如果无法安全修改配置，报告冲突并停止该 agent 的 hook 安装

如果某个 agent 当前版本无法确认 hook 配置格式或 hook 执行时机，桌面工具应该标记为
`不支持自动安装` 或 `配置冲突`，而不是猜测写入。

### 6. 三类 Agent 的真实 Hook 与 Skill 目录能力调研

本节记录 MVP 内置 agent 的实际能力边界。Skills Manager 的默认目录和 hook 安装策略必须以这些结论为准，
不能只按“配置根目录下都有 `skills` 子目录”推断。

| Agent | 已确认的 skill 发现目录 | 已确认的 hook / 插件能力 | 是否适合作为同次启动前置 reconcile | MVP 策略 |
| --- | --- | --- | --- | --- |
| Codex | 官方 manual 记录的用户目录是 `$HOME/.agents/skills`；仓库目录是从当前工作目录到仓库根目录沿途的 `.agents/skills`；管理员目录是 `/etc/codex/skills`；Codex 明确支持 symlinked skill folders。当前 Codex 自带 skill installer 还会安装到 `$CODEX_HOME/skills`，未设置 `CODEX_HOME` 时通常是 `~/.codex/skills`。 | hook 可来自 `~/.codex/hooks.json`、`~/.codex/config.toml`、项目 `.codex/hooks.json`、项目 `.codex/config.toml` 和插件；事件包含 `SessionStart`，matcher 可匹配 `startup`、`resume`、`clear`、`compact`。 | 不应默认视为适合。官方文档确认 `SessionStart` 存在，但未确认它一定早于 skill discovery。 | Codex 不使用单一硬编码默认目录。MVP 应检测 `$CODEX_HOME/skills`、`~/.codex/skills`、`~/.agents/skills` 的存在和当前 Codex 版本行为，并让用户选择管理目录。MVP 的 hook 时机选择 `SessionStart`；除非后续针对具体 Codex 版本验证了 `SessionStart` 早于 skill discovery，否则默认依赖桌面工具或 CLI 的“立即应用”。 |
| Claude Code | 默认用户目录是 `~/.claude/skills`；`CLAUDE_CONFIG_DIR` 可覆盖 `~/.claude`；项目目录是 `.claude/skills`；源码中也存在 managed skill 目录。 | hook 配置在 Claude settings 中；已确认事件包括 `Setup`、`SessionStart`、`InstructionsLoaded`、`ConfigChange`、`PreToolUse`、`Stop` 等。 | 需要版本验证。源码显示 setup 流程和 commands/skills 加载存在并行路径，不能仅凭 `Setup` 或 `SessionStart` 名称断定其早于 skill discovery。 | Claude Code 默认链接目录使用 `~/.claude/skills`。MVP 可检测 hook 配置和版本，但只有验证当前版本执行顺序后才自动安装前置 reconcile hook；否则依赖“立即应用”。 |
| OpenCode | 默认全局目录是 `~/.config/opencode/skills`；项目目录是 `.opencode/skills`；同时兼容 `.claude/skills` 和 `.agents/skills` 的全局与项目路径；配置中还支持额外 skill paths。 | OpenCode 支持插件，插件目录包括 `~/.config/opencode/plugins` 和 `.opencode/plugins`；插件事件包含 session、tool、command、shell、permission 等事件。 | 未找到可保证早于 skill discovery 的通用 hook。插件在启动时加载，但文档没有证明存在 pre-skill-discovery 时机。 | OpenCode 单独处理：优先使用 OpenCode 配置的额外 skill paths 直接指向共享源目录，避免不必要的 symlink；只有该方式不可用或用户选择时，才退回 symlink 到 `~/.config/opencode/skills`。 |

调研结论：

- 当前不能把“agent 有 hook/plugin 能力”直接等同于“可以在同次启动前完成 symlink reconcile”。
- 对 Codex、Claude Code、OpenCode，MVP 都应保留手动或桌面触发的 `reconcile` 作为可靠主路径。
- OpenCode 不强制复用通用 symlink 策略；应优先走 OpenCode 原生额外 skill paths 配置。
- 自动 hook 安装必须是按 agent、按版本逐个放开的能力；未验证执行顺序时只能展示检测结果和不可自动安装原因。
- 文档和实现中不得把 Codex 默认 skill 目录硬编码成单一路径；应同时识别 `$CODEX_HOME/skills`、
  `~/.codex/skills` 和 `~/.agents/skills` 这几类候选目录，并以运行时检测和用户确认结果为准。

资料来源：

- Codex：OpenAI Codex manual 的 Agent Skills 与 Hooks 章节。
- Claude Code：本地 `claude_code_src` 源码中的 setup、sessionStart、hooksConfigManager、loadSkillsDir 相关实现，以及本地 hook 文档。
- OpenCode：本地 `opencode-dev` 文档中的 skills、plugins 章节，以及 `packages/opencode/src/config/skills.ts`。
