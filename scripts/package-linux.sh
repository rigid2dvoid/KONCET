#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

version="$(tr -d '[:space:]' < VERSION)"
bundle="apps/desktop/build/linux/x64/release/bundle"
dist="$root/dist"
stage="$root/target/package"
appdir="$stage/KONCET-$version.AppDir"
release_name="KONCET-$version-linux-x86_64"

if [[ ! -x "$bundle/koncet" ]]; then
  echo "release bundle not found; run scripts/build-release.sh first" >&2
  exit 1
fi

rm -rf "$dist" "$appdir" "$stage/$release_name"
mkdir -p "$dist" "$stage" "$appdir/usr/bin" \
  "$appdir/usr/share/applications" \
  "$appdir/usr/share/icons/hicolor/256x256/apps"

# Portable tar.gz bundle.
mkdir -p "$stage/$release_name"
cp -a "$bundle/." "$stage/$release_name/"
cp apps/desktop/assets/licenses/GPL-3.0-or-later.txt "$stage/$release_name/LICENSE"
cp apps/desktop/assets/licenses/THIRD_PARTY_NOTICES.md "$stage/$release_name/THIRD_PARTY_NOTICES.md"
tar -C "$stage" -czf "$dist/$release_name.tar.gz" "$release_name"

# AppImage tree. Flutter resolves data/lib relative to the executable.
cp -a "$bundle/." "$appdir/usr/bin/"
cp apps/desktop/packaging/AppRun "$appdir/AppRun"
chmod +x "$appdir/AppRun"
cp apps/desktop/packaging/koncet.png "$appdir/koncet.png"
cp apps/desktop/packaging/koncet.png "$appdir/.DirIcon"
cp apps/desktop/packaging/koncet.png \
  "$appdir/usr/share/icons/hicolor/256x256/apps/koncet.png"
cp apps/desktop/packaging/dev.koncet.koncet.desktop \
  "$appdir/usr/share/applications/dev.koncet.koncet.desktop"
cp apps/desktop/packaging/dev.koncet.koncet.desktop \
  "$appdir/dev.koncet.koncet.desktop"

appimagetool="${APPIMAGETOOL:-$root/target/tools/appimagetool}"
if [[ ! -x "$appimagetool" ]]; then
  mkdir -p "$(dirname "$appimagetool")"
  echo "appimagetool not found; downloading the continuous AppImage build" >&2
  curl -fsSL \
    "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage" \
    -o "$appimagetool"
  chmod +x "$appimagetool"
fi

ARCH=x86_64 "$appimagetool" --appimage-extract-and-run --no-appstream \
  "$appdir" "$dist/$release_name.AppImage"

cp apps/desktop/assets/licenses/THIRD_PARTY_NOTICES.md "$dist/THIRD_PARTY_NOTICES.md"
cp apps/desktop/assets/licenses/GPL-3.0-or-later.txt "$dist/LICENSE"
./scripts/generate-sbom.py "$dist/$release_name-sbom.cdx.json"
(
  cd "$dist"
  sha256sum "$release_name.AppImage" "$release_name.tar.gz" \
    "$release_name-sbom.cdx.json" THIRD_PARTY_NOTICES.md LICENSE \
    > SHA256SUMS
)
echo "artifacts written to $dist"
ls -lh "$dist"
