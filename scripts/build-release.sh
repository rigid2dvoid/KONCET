#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

scripts/check-version.sh
if [[ -z "${KONCET_BUILD_COMMIT:-}" ]]; then
  if command -v git >/dev/null 2>&1 && git -C "$root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    KONCET_BUILD_COMMIT="$(git -C "$root" rev-parse --short=12 HEAD)"
  else
    KONCET_BUILD_COMMIT="development"
  fi
fi
export KONCET_BUILD_COMMIT

cd apps/desktop
flutter pub get
flutter build linux --release
