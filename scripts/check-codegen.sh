#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

files=(
  crates/koncet-ffi/src/frb_generated.rs
  apps/desktop/lib/src/rust/api.dart
  apps/desktop/lib/src/rust/dto.dart
  apps/desktop/lib/src/rust/error.dart
  apps/desktop/lib/src/rust/frb_generated.dart
  apps/desktop/lib/src/rust/frb_generated.io.dart
  apps/desktop/lib/src/rust/frb_generated.web.dart
)

before="$(sha256sum "${files[@]}")"
./scripts/codegen.sh
after="$(sha256sum "${files[@]}")"

if [[ "$before" != "$after" ]]; then
  echo "generated bridge is stale; commit the regenerated files" >&2
  diff <(printf '%s\n' "$before") <(printf '%s\n' "$after") >&2 || true
  exit 1
fi
echo "generated bridge is up to date"
