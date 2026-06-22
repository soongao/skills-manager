# Skills Manager

Skills Manager manages one active skills source across Codex, Claude Code, and OpenCode.

The MVP is implemented as a Rust core, a CLI, and a Tauri desktop shell. The desktop app runs as a
menu bar / tray utility with a compact status panel and a separate settings window. It reuses the
same core logic for source scanning, config, status, reconcile, detection, hook status, and OpenCode
native `skills.paths` integration.

## Workspace

```text
crates/skills-manager-core  Core filesystem, state, reconcile, cache, sync, detection, hook status
crates/skills-manager-cli   CLI for desktop automation, hooks, and manual operation
apps/desktop                Tauri desktop app with a Vite/vanilla frontend
scripts/                    Shell wrappers for CLI, desktop dev, checks, and smoke tests
doc/requirements.md         Requirements index
doc/requirements/           Split requirements documents
```

## Source Layout

The active source root must contain first-level skill directories:

```text
<sourceRoot>/
  .skills-manager/
    repository.json
  skills/
    <skillId>/
      ...
```

Skills Manager treats each `skills/<skillId>` directory as an opaque package. It does not parse or
rewrite skill contents.

## Build And Test

```bash
./scripts/check.sh
```

Run the CLI through the project wrapper:

```bash
./scripts/cli.sh help
```

Run the desktop app. It starts in the background; use the menu bar / tray icon to open the panel or
settings window.

```bash
./scripts/dev-desktop.sh
```

Run a local smoke test against temporary source/config directories:

```bash
./scripts/smoke.sh
```

## Basic CLI Flow

Initialize config in the default `~/.skills-manager` directory:

```bash
./scripts/cli.sh init-config --source-root /path/to/shared-skills --json
```

For testing or explicit agent path selection:

```bash
./scripts/cli.sh init-config \
  --source-root /path/to/shared-skills \
  --claude-code-skills-dir /tmp/claude-skills \
  --json
```

Inspect and apply local state:

```bash
./scripts/cli.sh scan --json
./scripts/cli.sh status --agent claude-code --json
./scripts/cli.sh reconcile --agent claude-code --json
```

Manage config explicitly:

```bash
./scripts/cli.sh config show --json
./scripts/cli.sh config set-agent --agent claude-code --skills-dir /path/to/claude/skills --json
./scripts/cli.sh config enable --agent claude-code --skill design-clarifier --json
./scripts/cli.sh config disable --agent claude-code --skill design-clarifier --json
```

When source skills are added later, update managed agents so new skills are enabled by default:

```bash
./scripts/cli.sh refresh-config --json
```

## OpenCode

OpenCode is handled separately. Prefer its native `skills.paths` config:

```bash
./scripts/cli.sh opencode ensure-path --json
```

Only plain `opencode.json` is modified. JSONC or unknown config structures are reported as conflicts,
so the symlink fallback can be used instead.

## Remote Sync

Remote mode uses `sync-cache`, not sshfs. For `push-local-to-remote`, Skills Manager
first syncs this Mac's source into the remote cache, then connects over SSH and links
the enabled skills into each configured remote agent folder. Existing non-managed
targets are skipped as conflicts and reported to the user.

```bash
./scripts/cli.sh remote sync --environment devbox --direction push-local-to-remote --plan --json
./scripts/cli.sh remote sync --environment devbox --direction push-local-to-remote --json
./scripts/cli.sh remote cli-status --environment devbox --json
```

`rsync --delete` only runs after the target cache marker is verified.

## Hook Status

Hook auto-install is intentionally conservative:

```bash
./scripts/cli.sh hook status --json
./scripts/cli.sh hook status --agent codex --json
```

The current MVP reports hook capability and config candidates, but does not auto-install hooks until
the concrete agent version is verified to run session-start hooks before skill discovery.

## JSON Output

Automation commands support `--json` and return:

- `schemaVersion`
- `ok`
- `status`
- `command`
- `runId`
- `startedAt`
- `endedAt`
- `summary`
- `actions`
- `warnings`
- `errors`
- `result`

Write operations record JSON Lines logs and run records under the configured home:

```text
~/.skills-manager/logs/skills-manager.log
~/.skills-manager/runs/
```

Supported agent IDs are `codex`, `claude-code`, and `opencode`.
