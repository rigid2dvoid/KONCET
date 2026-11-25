#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

scripts/check-version.sh

if command -v git >/dev/null 2>&1 && git -C "$root" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  dirty=0
  if [[ -n "$(git -C "$root" status --porcelain)" ]]; then
    dirty=1
  fi
  if [[ "$dirty" == "1" && "${KONCET_ALLOW_DIRTY:-0}" != "1" ]]; then
    cat >&2 <<'EOF'
Refusing to build release artifacts from a dirty working tree.

Commit or discard the changes first. For a local development bundle only, set:
  KONCET_ALLOW_DIRTY=1 ./scripts/build-release.sh
EOF
    exit 1
  fi
  if [[ -z "${KONCET_BUILD_COMMIT:-}" ]]; then
    KONCET_BUILD_COMMIT="$(git -C "$root" rev-parse --short=12 HEAD)"
    [[ "$dirty" == "1" ]] && KONCET_BUILD_COMMIT="$KONCET_BUILD_COMMIT-dirty"
  fi
  if [[ -z "${KONCET_BUILD_DATE:-}" ]]; then
    if [[ "$dirty" == "1" ]]; then
      KONCET_BUILD_DATE="$(date -u '+%Y-%m-%dT%H:%M:%SZ')"
    else
      KONCET_BUILD_DATE="$(git -C "$root" show -s --format=%cI HEAD)"
    fi
  fi
else
  KONCET_BUILD_COMMIT="${KONCET_BUILD_COMMIT:-development}"
  KONCET_BUILD_DATE="${KONCET_BUILD_DATE:-$(date -u '+%Y-%m-%dT%H:%M:%SZ')}"
fi
export KONCET_BUILD_COMMIT KONCET_BUILD_DATE

version="$(tr -d '[:space:]' < "$root/VERSION")"
echo "Building KONCET $version"
echo "  commit: $KONCET_BUILD_COMMIT"
echo "  date:   $KONCET_BUILD_DATE"

cd apps/desktop
flutter pub get
flutter build linux --release
