#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/skills-manager-smoke.XXXXXX")"
trap 'rm -rf "$TMP_ROOT"' EXIT

SOURCE_ROOT="$TMP_ROOT/source"
CONFIG_HOME="$TMP_ROOT/config"
CLAUDE_SKILLS_DIR="$TMP_ROOT/claude-skills"

mkdir -p "$SOURCE_ROOT/skills/design-clarifier" "$SOURCE_ROOT/skills/review-helper" "$CLAUDE_SKILLS_DIR"
printf '# Design Clarifier\n' > "$SOURCE_ROOT/skills/design-clarifier/SKILL.md"
printf '# Review Helper\n' > "$SOURCE_ROOT/skills/review-helper/SKILL.md"

cd "$ROOT_DIR"

cargo run -p skills-manager-cli -- init-config \
  --source-root "$SOURCE_ROOT" \
  --config-home "$CONFIG_HOME" \
  --claude-code-skills-dir "$CLAUDE_SKILLS_DIR" \
  --json >/dev/null

cargo run -p skills-manager-cli -- status \
  --config-home "$CONFIG_HOME" \
  --agent claude-code \
  --json >/dev/null

cargo run -p skills-manager-cli -- reconcile \
  --config-home "$CONFIG_HOME" \
  --agent claude-code \
  --json >/dev/null

test -L "$CLAUDE_SKILLS_DIR/design-clarifier"
test -L "$CLAUDE_SKILLS_DIR/review-helper"

echo "smoke ok"
