# Skills Manager Requirements

这个目录保存 Skills Manager 的 MVP 需求。详细内容已经按主题拆分到 `requirements/` 子目录，避免单个文档过长。

## 阅读顺序

1. [目标与能力范围](./requirements/01-goals.md)
2. [状态模型](./requirements/02-state-model.md)
3. [实现契约](./requirements/03-implementation-contract.md)
4. [核心约束](./requirements/04-constraints.md)
5. [冲突处理](./requirements/05-conflict-handling.md)
6. [推荐架构与范围](./requirements/06-architecture-and-scope.md)

## MVP 关键结论

- 同一时间只允许一个 `activeSourceProfileId`，所有 environment 使用同一个 active source。
- 启用状态按 `environment × skill × agent` 保存。
- 本地 agent 通过 symlink 或 agent 原生配置引用 active source。
- OpenCode 优先使用原生额外 skill paths，只有不可用或用户选择时才退回 symlink。
- 远程能力采用 `sync-cache`，支持 `push-local-to-remote` 和 `pull-remote-to-local`。
- 远程缓存不是事实来源，带删除语义的同步必须检测 cache marker。
- hook 时机选择 session start，但只有验证具体 agent 版本早于 skill discovery 后才自动安装。
- 冲突处理 MVP 只支持跳过并提示用户，不自动覆盖、接管或合并。

## 当前待确认

- Codex、Claude Code、OpenCode 的哪些具体版本可以证明 hook 早于 skill discovery。
