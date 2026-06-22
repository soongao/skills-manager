#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/skills-manager-smoke.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

SOURCE_ROOT="$TMP_ROOT/source"
CONFIG_HOME="$TMP_ROOT/config"
TEST_HOME="$TMP_ROOT/home"
CLAUDE_SKILLS_DIR="$TEST_HOME/claude-skills"

mkdir -p "$SOURCE_ROOT/skills/design-clarifier" "$SOURCE_ROOT/skills/review-helper" "$TEST_HOME"
printf '# Design Clarifier\n' > "$SOURCE_ROOT/skills/design-clarifier/SKILL.md"
printf '# Review Helper\n' > "$SOURCE_ROOT/skills/review-helper/SKILL.md"

cd "$ROOT_DIR"

cargo build -p skills-manager-cli >/dev/null
CLI_BIN="$ROOT_DIR/target/debug/skills-manager"

HOME="$TEST_HOME" "$CLI_BIN" init-config \
  --source-root "$SOURCE_ROOT" \
  --config-home "$CONFIG_HOME" \
  --claude-code-skills-dir '~/claude-skills' \
  --json >/dev/null

HOME="$TEST_HOME" "$CLI_BIN" status \
  --config-home "$CONFIG_HOME" \
  --agent claude-code \
  --json >/dev/null

HOME="$TEST_HOME" "$CLI_BIN" reconcile \
  --config-home "$CONFIG_HOME" \
  --agent claude-code \
  --json >/dev/null

test -L "$CLAUDE_SKILLS_DIR/design-clarifier"
test -L "$CLAUDE_SKILLS_DIR/review-helper"
test ! -e "$ROOT_DIR/~"

echo "smoke ok"
