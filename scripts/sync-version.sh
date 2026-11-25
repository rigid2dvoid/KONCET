#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
version="${1:-}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || {
  echo "usage: $0 MAJOR.MINOR.PATCH[-rc.N]" >&2
  exit 2
}

printf '%s\n' "$version" > "$root/VERSION"
sed -i -E "0,/^version = \".*\"/s//version = \"$version\"/" "$root/Cargo.toml"
sed -i -E "s/^version: .*/version: $version+1/" "$root/apps/desktop/pubspec.yaml"
sed -i -E "s/^version: .*/version: $version/" "$root/packages/koncet_ui/pubspec.yaml"

cd "$root"
cargo check -q
echo "version synced to $version"
