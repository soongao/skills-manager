#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo fmt --check
cargo test

cd "$ROOT_DIR/apps/desktop"
npm run build:web

cd "$ROOT_DIR"
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
