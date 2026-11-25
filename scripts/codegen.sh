#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

if ! command -v flutter_rust_bridge_codegen >/dev/null 2>&1; then
  echo "flutter_rust_bridge_codegen 2.13.0 is required." >&2
  echo "Install it with: cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked" >&2
  exit 1
fi

flutter_rust_bridge_codegen generate
cargo fmt --all
dart format apps/desktop/lib/src/rust packages/koncet_ui/lib apps/desktop/lib
