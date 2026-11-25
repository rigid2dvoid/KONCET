#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="$(tr -d '[:space:]' < "$root/VERSION")"

fail() {
  echo "version mismatch: $1" >&2
  exit 1
}

cargo_version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -1)"
[[ "$cargo_version" == "$version" ]] || fail "Cargo workspace is $cargo_version, VERSION is $version"

desktop_version="$(sed -n 's/^version: \(.*\)+.*/\1/p' "$root/apps/desktop/pubspec.yaml" | head -1)"
[[ "$desktop_version" == "$version" ]] || fail "desktop pubspec is $desktop_version, VERSION is $version"

ui_version="$(sed -n 's/^version: \(.*\)/\1/p' "$root/packages/koncet_ui/pubspec.yaml" | head -1)"
[[ "$ui_version" == "$version" ]] || fail "koncet_ui pubspec is $ui_version, VERSION is $version"

tag="${1:-}"
if [[ -n "$tag" && "$tag" != "v$version" ]]; then
  fail "tag $tag does not match v$version"
fi

echo "versions agree on $version"
