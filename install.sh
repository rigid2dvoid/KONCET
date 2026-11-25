#!/usr/bin/env bash
# KONCET user-local installer.
#
#   ./install.sh [ARTIFACT]          install an AppImage or .tar.gz
#   ./install.sh --bundle DIR        install an already-built Flutter bundle
#   ./install.sh --uninstall         remove installed files (data is kept)
#   ./install.sh --help              show this help
#
# Defaults to ~/.local, so no sudo is required. The installer never touches
# Docker, Kubernetes, network services, or the application's local data.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PREFIX="${PREFIX:-$HOME/.local}"
ARTIFACT=""
BUNDLE=""
ICON=""
INSTALL_DESKTOP=1
DO_UNINSTALL=0

BIN_DIR=""
LIB_DIR=""
APPS_DIR=""
ICON_DIR=""

usage() {
  sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'
}

die() {
  echo "install.sh: $*" >&2
  exit 1
}

warn() {
  echo "install.sh: warning: $*" >&2
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix)
      [[ $# -ge 2 ]] || die "--prefix requires a directory"
      PREFIX="$2"
      shift 2
      ;;
    --prefix=*)
      PREFIX="${1#*=}"
      shift
      ;;
    --bundle|--from-bundle)
      [[ $# -ge 2 ]] || die "$1 requires a directory"
      BUNDLE="$2"
      shift 2
      ;;
    --icon)
      [[ $# -ge 2 ]] || die "--icon requires a file"
      ICON="$2"
      shift 2
      ;;
    --no-desktop)
      INSTALL_DESKTOP=0
      shift
      ;;
    --uninstall)
      DO_UNINSTALL=1
      shift
      ;;
    -h|--help|help)
      usage
      exit 0
      ;;
    -*)
      die "unknown option: $1"
      ;;
    *)
      [[ -z "$ARTIFACT" ]] || die "only one artifact may be provided"
      ARTIFACT="$1"
      shift
      ;;
  esac
done

# Normalise a trailing slash, but keep the prefix relative if the user chose so.
PREFIX="${PREFIX%/}"
[[ -n "$PREFIX" ]] || die "prefix must not be empty"
case "$PREFIX" in
  /*) ;;
  *) PREFIX="$PWD/$PREFIX" ;;
esac

BIN_DIR="$PREFIX/bin"
LIB_DIR="$PREFIX/lib/koncet"
APPS_DIR="$PREFIX/share/applications"
ICON_DIR="$PREFIX/share/icons/hicolor/256x256/apps"
DESKTOP_FILE="$APPS_DIR/koncet.desktop"
ICON_FILE="$ICON_DIR/koncet.png"

refresh_caches() {
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APPS_DIR" >/dev/null 2>&1 || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
  fi
}

data_dir() {
  printf '%s\n' "${XDG_DATA_HOME:-$HOME/.local/share}/koncet"
}

print_path_hint() {
  case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *)
      echo
      echo "Note: $BIN_DIR is not on your PATH."
      echo "Add this to your shell profile:"
      echo "  export PATH=\"$BIN_DIR:\$PATH\""
      ;;
  esac
}

if [[ "$DO_UNINSTALL" == "1" ]]; then
  rm -f "$BIN_DIR/koncet"
  rm -rf "$LIB_DIR"
  rm -f "$DESKTOP_FILE"
  rm -f "$ICON_FILE"
  refresh_caches
  echo "KONCET has been removed from $PREFIX."
  echo "Application data was kept at: $(data_dir)"
  echo "To delete it permanently: rm -rf \"$(data_dir)\""
  exit 0
fi

tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/koncet-install.XXXXXX")"
cleanup() {
  rm -rf "$tmp_dir"
}
trap cleanup EXIT

verify_artifact() {
  local file="$1"
  local dir base
  dir="$(cd "$(dirname "$file")" && pwd)"
  base="$(basename "$file")"
  if [[ -f "$dir/SHA256SUMS" ]]; then
    if grep -qE "[[:space:]]+\\*?${base//./\\.}$" "$dir/SHA256SUMS"; then
      (cd "$dir" && sha256sum -c --ignore-missing SHA256SUMS)
    else
      warn "SHA256SUMS has no entry for $base; skipping checksum verification"
    fi
  else
    warn "SHA256SUMS was not found next to $base; skipping checksum verification"
  fi
}

# Pick a single release artifact from the usual locations.
detect_artifact() {
  local candidates=()
  local dir file
  for dir in "$PWD" "$SCRIPT_DIR" "$SCRIPT_DIR/dist"; do
    [[ -d "$dir" ]] || continue
    while IFS= read -r -d '' file; do
      candidates+=("$file")
    done < <(find "$dir" -maxdepth 1 -type f \
      \( -name 'KONCET-*-linux-x86_64.AppImage' -o -name 'KONCET-*-linux-x86_64.tar.gz' \) \
      -print0 | sort -z)
  done
  [[ ${#candidates[@]} -gt 0 ]] || return 1

  # Remove duplicates while preserving order.
  mapfile -t candidates < <(printf '%s\n' "${candidates[@]}" | awk '!seen[$0]++')

  local appimages=() tarballs=()
  for file in "${candidates[@]}"; do
    case "$file" in
      *.AppImage) appimages+=("$file") ;;
      *.tar.gz) tarballs+=("$file") ;;
    esac
  done

  if [[ ${#appimages[@]} -eq 1 ]]; then
    ARTIFACT="${appimages[0]}"
    return 0
  fi
  if [[ ${#appimages[@]} -gt 1 ]]; then
    echo "Multiple KONCET AppImages were found. Pass one explicitly:" >&2
    printf '  %s\n' "${appimages[@]}" >&2
    return 2
  fi
  if [[ ${#tarballs[@]} -eq 1 ]]; then
    ARTIFACT="${tarballs[0]}"
    return 0
  fi
  echo "Multiple KONCET archives were found. Pass one explicitly:" >&2
  printf '  %s\n' "${tarballs[@]}" >&2
  return 2
}

resolve_icon_from_appimage() {
  local image="$1"
  local probe="$tmp_dir/icon-probe.AppImage"
  local extracted="$tmp_dir/icon-extract"
  mkdir -p "$extracted"
  cp "$image" "$probe"
  chmod +x "$probe"
  if (cd "$extracted" && "$probe" --appimage-extract .DirIcon >/dev/null 2>&1); then
    if [[ -f "$extracted/squashfs-root/.DirIcon" ]]; then
      printf '%s\n' "$extracted/squashfs-root/.DirIcon"
      return 0
    fi
    if [[ -f "$extracted/squashfs-root/koncet.png" ]]; then
      printf '%s\n' "$extracted/squashfs-root/koncet.png"
      return 0
    fi
  fi
  return 1
}

write_desktop_entry() {
  local icon_name="$1"
  local icon_value
  mkdir -p "$APPS_DIR" "$ICON_DIR"
  if [[ -n "$icon_name" && -f "$icon_name" ]]; then
    install -m 0644 "$icon_name" "$ICON_FILE"
    icon_value="$ICON_FILE"
  else
    icon_value="applications-development"
    warn "no application icon was found; using the desktop theme fallback"
  fi

  cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=KONCET
GenericName=Deployment simulation workbench
Comment=Deterministic deployment simulation for Linux
Exec="$BIN_DIR/koncet"
Icon=$icon_value
Terminal=false
Categories=Development;
Keywords=simulation;deployment;build;operations;
StartupWMClass=dev.koncet.koncet
EOF
  chmod 0644 "$DESKTOP_FILE"
  refresh_caches
}

write_wrapper() {
  mkdir -p "$BIN_DIR"
  cat > "$BIN_DIR/koncet" <<EOF
#!/bin/sh
exec "$LIB_DIR/koncet" "\$@"
EOF
  chmod 0755 "$BIN_DIR/koncet"
}

finish_install() {
  print_path_hint
  echo
  echo "KONCET installed under $PREFIX"
  echo "  Launch:  koncet"
  echo "  Data:    $(data_dir)"
  if [[ "$INSTALL_DESKTOP" == "1" ]]; then
    echo "  Menu:    KONCET is available in the application menu"
  fi
}

install_bundle() {
  local source="$1"
  local icon_path="${2:-}"
  [[ -x "$source/koncet" ]] || die "not a Flutter release bundle (missing executable): $source"
  mkdir -p "$LIB_DIR" "$BIN_DIR"
  rm -rf "$LIB_DIR"
  cp -a "$source/." "$LIB_DIR/"
  chmod +x "$LIB_DIR/koncet"
  write_wrapper

  if [[ -z "$icon_path" && -f "$source/koncet.png" ]]; then
    icon_path="$source/koncet.png"
  fi
  if [[ -z "$icon_path" ]]; then
    icon_path="$SCRIPT_DIR/apps/desktop/packaging/koncet.png"
    [[ -f "$icon_path" ]] || icon_path=""
  fi
  if [[ "$INSTALL_DESKTOP" == "1" ]]; then
    write_desktop_entry "$icon_path"
  fi
  finish_install
}

install_appimage() {
  local image="$1"
  verify_artifact "$image"
  mkdir -p "$BIN_DIR"
  rm -rf "$LIB_DIR"
  mkdir -p "$LIB_DIR"
  install -m 0755 "$image" "$LIB_DIR/koncet.AppImage"
  cat > "$BIN_DIR/koncet" <<EOF
#!/bin/sh
# Prefer direct AppImage mounting; fall back to extraction when FUSE is absent.
if [ -r /dev/fuse ]; then
  exec "$LIB_DIR/koncet.AppImage" "\$@"
else
  exec "$LIB_DIR/koncet.AppImage" --appimage-extract-and-run "\$@"
fi
EOF
  chmod 0755 "$BIN_DIR/koncet"

  local icon_path="${ICON:-}"
  if [[ -z "$icon_path" && -f "$(dirname "$image")/koncet.png" ]]; then
    icon_path="$(dirname "$image")/koncet.png"
  fi
  if [[ -z "$icon_path" ]]; then
    icon_path="$(resolve_icon_from_appimage "$image" || true)"
  fi
  if [[ "$INSTALL_DESKTOP" == "1" ]]; then
    write_desktop_entry "$icon_path"
  fi
  finish_install
}

install_tar() {
  local archive="$1"
  verify_artifact "$archive"
  local extract_dir="$tmp_dir/archive"
  mkdir -p "$extract_dir"
  tar -xzf "$archive" -C "$extract_dir"
  local executable
  executable="$(find "$extract_dir" -maxdepth 3 -type f -name koncet -perm -u+x | head -n 1)"
  [[ -n "$executable" ]] || die "the archive does not contain a KONCET executable"
  local bundle_dir
  bundle_dir="$(dirname "$executable")"
  local icon_path="${ICON:-}"
  if [[ -z "$icon_path" && -f "$bundle_dir/koncet.png" ]]; then
    icon_path="$bundle_dir/koncet.png"
  fi
  if [[ -z "$icon_path" && -f "$(dirname "$archive")/koncet.png" ]]; then
    icon_path="$(dirname "$archive")/koncet.png"
  fi
  install_bundle "$bundle_dir" "$icon_path"
}

if [[ -n "$BUNDLE" ]]; then
  install_bundle "$BUNDLE" "$ICON"
else
  if [[ -z "$ARTIFACT" ]]; then
    detect_artifact || {
      status=$?
      if [[ "$status" == "1" ]]; then
        cat >&2 <<EOF
No KONCET release artifact was found.

Download the AppImage, .tar.gz, and SHA256SUMS from GitHub Releases, then run:
  ./install.sh KONCET-<version>-linux-x86_64.AppImage

Or build and install from source:
  ./run.sh --install
EOF
      fi
      exit 1
    }
  fi

  [[ -e "$ARTIFACT" ]] || die "artifact not found: $ARTIFACT"
  if [[ -d "$ARTIFACT" ]]; then
    install_bundle "$ARTIFACT" "$ICON"
  else
    case "$ARTIFACT" in
      *.AppImage) install_appimage "$ARTIFACT" ;;
      *.tar.gz) install_tar "$ARTIFACT" ;;
      *) die "unsupported artifact (expected .AppImage or .tar.gz): $ARTIFACT" ;;
    esac
  fi
fi
