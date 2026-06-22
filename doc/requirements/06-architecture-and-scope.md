# 推荐架构与范围

## 推荐架构

MVP 技术选型采用 Tauri + Rust。Rust 负责文件系统操作、symlink、配置读写、CLI、
远程 SSH/rsync 同步、hook 安装和能力检测；Tauri 前端负责桌面管理界面。

Desktop 和 CLI 应共用同一套 Rust 核心逻辑，避免两套 reconcile、冲突检测或状态判断规则。

```text
本地源仓库
  <shared-skills-root>/skills

共享仓库元信息
  <shared-skills-root>/.skills-manager/repository.json

桌面管理层
  管理 active source profile、environment、agent 配置、enabledSkillIds 和冲突决策

Rust 核心库
  提供扫描、状态计算、reconcile、hook 安装、远程同步和能力检测

用户配置目录
  保存当前机器的期望状态、agent 路径、远程主机、同步方向和缓存配置

本地 agent 引用层
  <codex-skills-dir>/<skillId>                -> 本地源目录
  ~/.claude/skills/<skillId>                  -> 本地源目录
  OpenCode 优先通过额外 skill paths 指向本地源目录
  ~/.config/opencode/skills/<skillId>         -> 本地源目录，作为 OpenCode 退回方案

本地 reconciler
  由桌面工具主动触发，或由本地 agent hook 调用 skills-manager CLI 触发

Hook 安装器
  检测 Codex、Claude Code、OpenCode 的 hook 能力
  只为已确认可早于 skill discovery 的 agent 版本安装或卸载 Skills Manager hook

远程同步层
  由 Skills Manager 通过 SSH/rsync 管理
  push-local-to-remote: 本机源目录 -> 远程缓存目录
  pull-remote-to-local: 远程源目录 -> 本机缓存目录

远程 agent 引用层
  <codex-skills-dir>/<skillId>                -> agent 所在机器的缓存目录
  ~/.claude/skills/<skillId>                  -> agent 所在机器的缓存目录
  OpenCode 优先通过额外 skill paths 指向 agent 所在机器的缓存目录
  ~/.config/opencode/skills/<skillId>         -> agent 所在机器的缓存目录，作为 OpenCode 退回方案

远程 reconciler
  由 agent 所在机器的 hook 调用 skills-manager CLI 触发，使用该机器可见的缓存路径
```

## 非目标

- 不用一个共享 symlink 替换整个 agent skill 目录。
- 不把共享 skills 复制到每个 agent 目录作为主要机制。
- 不静默覆盖已经存在的 agent 本地 skills。
- 不自动合并发生冲突的 skill 目录。
- 初始设计不依赖 NFS、sshfs 或 overlay filesystem。
- 不要求桌面工具进程直接完成所有 symlink 操作。
- hook 不做交互式冲突决策。
- hook 不直接实现链接逻辑，应调用 Skills Manager CLI。
- hook reconcile 失败不阻止 agent 启动。
- 禁用 skill 时不删除未知来源的同名文件、目录或 symlink。
- 不把机器相关路径、远程主机、缓存目录或用户启用状态写入共享源目录。
- 不解析、校验、修改或转换 skill 目录内部内容。
- MVP 不支持 agent 专属适配层或 wrapper 内容生成。
- 不猜测写入无法确认格式或时机的 agent hook 配置。
- 不在 Windows symlink 权限不足时静默复制 skill 目录作为替代。
- 不保存或托管 SSH 密码、私钥或 passphrase。
- MVP 不支持同时管理多个共享 skill 源目录。
- 不由本地桌面工具通过 SSH 代替远程 hook 执行远程 symlink reconcile。
- MVP 不自动安装或升级远程机器上的 Skills Manager CLI。
- 不在主版本不兼容时执行写入类操作。
- 日志和运行记录不写入共享源目录。
- MVP 不提供一键回滚功能；冲突只跳过并提示用户手动处理。

## 待确认问题

- Codex、Claude Code、OpenCode 的哪些具体版本可以证明 hook 早于 skill discovery？
