# Development Status

## Current Implementation

- Created a Rust workspace with `skills-manager-core` and `skills-manager-cli`.
- Implemented core models for source profiles, environments, agents, skills, config, and state.
- Implemented source scanning for `<sourceRoot>/skills/<skillId>`.
- Implemented repository metadata initialization and update at `<sourceRoot>/.skills-manager/repository.json`.
- Implemented local status calculation for disabled, pending, enabled, conflict, and invalid states.
- Implemented conservative local symlink reconcile for enabled skills.
- Implemented state-backed managed link tracking, including removal of disabled managed symlinks only when previously registered.
- Implemented cache marker initialization and verification.
- Implemented remote `sync-cache` rsync plan generation and execution for:
  - `push-local-to-remote`
  - `pull-remote-to-local`
- Implemented local and remote cache marker preflight checks before destructive sync.
- Implemented machine and agent capability detection for:
  - OS and architecture
  - symlink capability
  - `ssh`
  - `rsync`
  - Codex candidate skill dirs
  - Claude Code candidate skill dirs
  - OpenCode candidate skill dirs
- Implemented hook status reporting for Codex, Claude Code, and OpenCode.
- Hook installation remains intentionally blocked unless a future agent/version verification proves session-start hook timing is safe.
- Implemented OpenCode native `skills.paths` integration for plain `opencode.json`.
- Implemented config-driven CLI commands with unified `--json` output.
- Split the CLI into focused modules:
  - `args.rs` for argument parsing
  - `output.rs` for run context, JSON output, logs, and human output
  - `help.rs` for help text
  - `util.rs` for shared CLI helpers
  - `commands/general.rs` for general local commands
  - `commands/config.rs` for config mutation commands
  - `commands/remote.rs` for remote sync/status commands
- Implemented JSON Lines logs and per-run JSON records under the configured Skills Manager home.
- Added a Tauri desktop app under `apps/desktop`.
- Desktop app currently supports:
  - menu bar / tray background mode
  - compact status panel
  - separate full settings window
  - close-to-hide window behavior
  - initialize source/config
  - view source, discovered skills, agents, statuses, hooks, and dependency detection
  - edit local agent skill directories
  - toggle skill enablement per local agent
  - run local reconcile
  - configure remote source pull mode
  - configure remote environment push mode
  - plan and run remote cache sync from settings
  - test remote `skills-manager` CLI availability over SSH
  - call OpenCode native `skills.paths` integration from the Rust backend
- Added project shell wrappers:
  - `./scripts/cli.sh` for CLI execution
  - `./scripts/dev-desktop.sh` for desktop development
  - `./scripts/check.sh` for Rust and desktop verification
  - `./scripts/smoke.sh` for a temporary end-to-end local smoke test

## CLI Commands

Implemented commands:

```bash
skills-manager detect [--json]
skills-manager version [--json]
skills-manager init-config --source-root <path> [--config-home <path>] [--<agent>-skills-dir <path>] [--json]
skills-manager init-repo --source-root <path> [--name <name>] [--json]
skills-manager refresh-config [--config-home <path>] [--json]
skills-manager config show [--json]
skills-manager config set-local-source --source-root <path> [--source-profile-id <id>] [--json]
skills-manager config set-remote-source --host <host> --user <user> --remote-source-root <path> --local-cache-root <path> [--json]
skills-manager config set-agent --agent <agent> --skills-dir <path> [--environment <id>] [--json]
skills-manager config enable --agent <agent> --skill <skillId> [--environment <id>] [--json]
skills-manager config disable --agent <agent> --skill <skillId> [--environment <id>] [--json]
skills-manager config add-remote-env --environment <id> --host <host> --user <user> [--direction <direction>] [--json]
skills-manager scan [--source-root <path>] [--json]
skills-manager status [--agent <agent>] [--environment <id>] [--json]
skills-manager reconcile [--agent <agent>] [--environment <id>] [--plan] [--json]
skills-manager remote sync [--environment <id>] [--direction <direction>] [--plan] [--json]
skills-manager remote status [--environment <id>] [--json]
skills-manager remote cli-status --environment <id> [--json]
skills-manager hook status [--agent <agent>] [--json]
skills-manager hook install --agent <agent> [--json]
skills-manager opencode ensure-path [--skills-root <path>] [--config-path <path>] [--json]
skills-manager cache init <cache-root> <repo-id> <source-profile-id>
skills-manager cache verify <cache-root> <repo-id> <source-profile-id>
```

Legacy manual testing forms still work:

```bash
skills-manager scan <source-root>
skills-manager status <source-root> <agent> <skills-dir>
skills-manager reconcile <source-root> <agent> <skills-dir>
```

## Verification

Rust is available in the current environment:

- `cargo 1.96.0`
- `rustc 1.96.0`
- `rustfmt 1.9.0-stable`

The following commands have been run successfully:

```bash
./scripts/check.sh
./scripts/smoke.sh
./scripts/dev-desktop.sh
```

Current test result: 13 unit tests passed.

An end-to-end smoke test was also run using a temporary source root, temporary config home, and
explicit Claude Code skills directory under `/private/tmp`. It verified:

- `init-config --json`
- `status --agent claude-code --json`
- `reconcile --agent claude-code --json`
- symlink creation for two skills
- `state.json` creation
- log file creation
- run record creation
- config enable/disable commands
- removal and recreation behavior through reconcile

## Remaining Work

- Add richer UI-facing issue aggregation.
- Add richer desktop logs/runs viewer.
- Add safe hook installer implementations only after concrete agent/version timing verification.
