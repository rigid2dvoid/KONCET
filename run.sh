#!/usr/bin/env bash
# KONCET launcher for a fresh checkout.
#
#   ./run.sh              # fetch deps and launch the development app
#   ./run.sh --release    # build the release bundle and launch it
#   ./run.sh --install    # build release and install under ~/.local
#   ./run.sh --check      # verify the development toolchain
#   ./run.sh --help       # show this help
#
# `just` and flutter_rust_bridge_codegen are not required here. The generated
# bridge is committed; only Rust and Flutter are needed to run the app.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

MODE="dev"
PASSTHROUGH=()

usage() {
  cat <<'EOF'
KONCET launcher for a fresh checkout.

  ./run.sh              # fetch deps and launch the development app
  ./run.sh --release    # build the release bundle and launch it
  ./run.sh --install    # build release and install under ~/.local
  ./run.sh --check      # verify the development toolchain
  ./run.sh --help       # show this help

`just` and flutter_rust_bridge_codegen are not required here. The generated
bridge is committed; only Rust and Flutter are needed to run the app.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dev)
      MODE="dev"
      shift
      ;;
    --release)
      MODE="release"
      shift
      ;;
    --install)
      MODE="install"
      shift
      ;;
    --check|check)
      MODE="check"
      shift
      ;;
    -h|--help|help)
      usage
      exit 0
      ;;
    --)
      shift
      PASSTHROUGH+=("$@")
      break
      ;;
    *)
      echo "Unknown option: $1" >&2
      echo "Run ./run.sh --help for usage." >&2
      exit 2
      ;;
  esac
done

# Make common per-user installations work without shell-profile edits.
if ! command -v flutter >/dev/null 2>&1; then
  for candidate in \
    "${FLUTTER_HOME:-}" \
    "$HOME/.local/share/flutter" \
    "$HOME/flutter" \
    "/opt/flutter"; do
    if [[ -n "$candidate" && -x "$candidate/bin/flutter" ]]; then
      export PATH="$candidate/bin:$PATH"
      break
    fi
  done
fi
if ! command -v cargo >/dev/null 2>&1 && [[ -x "$HOME/.cargo/bin/cargo" ]]; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

FLUTTER="${FLUTTER_BIN:-flutter}"
CARGO="${CARGO_BIN:-cargo}"

missing=()
for tool in "$CARGO" "$FLUTTER"; do
  command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done
for tool in pkg-config cmake ninja clang; do
  command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
done

gtk_ok=true
if command -v pkg-config >/dev/null 2>&1; then
  pkg-config --exists gtk+-3.0 x11 xi || gtk_ok=false
fi

print_native_hint() {
  if [[ -r /etc/os-release ]]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    case "${ID:-} ${ID_LIKE:-}" in
      *debian*|*ubuntu*)
        echo "  sudo apt install build-essential clang cmake ninja-build pkg-config libgtk-3-dev libx11-dev libxi-dev"
        ;;
      *fedora*|*rhel*)
        echo "  sudo dnf install clang cmake ninja-build pkgconf-pkg-config gtk3-devel libX11-devel libXi-devel"
        ;;
      *arch*)
        echo "  sudo pacman -S --needed base-devel clang cmake ninja pkgconf gtk3 libx11 libxi"
        ;;
    esac
  fi
}

if (( ${#missing[@]} > 0 )) || [[ "$gtk_ok" != true ]]; then
  echo "KONCET cannot start: the development toolchain is incomplete." >&2
  echo
  if (( ${#missing[@]} > 0 )); then
    printf 'Missing command(s): %s\n' "${missing[*]}" >&2
  fi
  if [[ "$gtk_ok" != true ]]; then
    echo "Missing GTK 3 development headers (pkg-config gtk+-3.0)." >&2
  fi
  echo
  echo "Install one-time prerequisites:" >&2
  echo "  - Rust 1.85+ with cargo: https://rustup.rs" >&2
  echo "  - Flutter 3.50.x stable (Dart 3.14): https://docs.flutter.dev/get-started/install/linux" >&2
  echo "  - Native packages for your distribution:" >&2
  print_native_hint >&2
  echo
  echo "Then add Flutter to PATH, for example:" >&2
  echo "  export PATH=\"\$HOME/.local/share/flutter/bin:\$PATH\"" >&2
  echo
  echo "See docs/engineering/toolchain.md for verified versions." >&2
  exit 1
fi

version_line() {
  "$1" --version 2>/dev/null | head -n 1 || true
}

if [[ "$MODE" == "check" ]]; then
  echo "KONCET development toolchain is ready."
  echo "  Rust:    $(version_line "$CARGO")"
  echo "  Flutter: $(version_line "$FLUTTER")"
  echo "  Rust bridge codegen: not required (generated code is committed)"
  echo
  echo "Next: ./run.sh"
  exit 0
fi

# Flutter's Linux build invokes cargo through CMake; fetching Dart deps first
# gives clearer errors and is safe to repeat.
"$FLUTTER" pub get --directory "$ROOT/apps/desktop" >/dev/null
"$FLUTTER" pub get --directory "$ROOT/packages/koncet_ui" >/dev/null

build_release() {
  echo "Building KONCET release bundle…"
  "$ROOT/scripts/build-release.sh"
}

if [[ "$MODE" == "install" ]]; then
  build_release
  "$ROOT/install.sh" \
    --bundle "$ROOT/apps/desktop/build/linux/x64/release/bundle" \
    --icon "$ROOT/apps/desktop/packaging/koncet.png"
  echo
  echo "Installed. Launch from a terminal with:  koncet"
  echo "Or open KONCET from your application menu."
  exit 0
fi

if [[ "$MODE" == "release" ]]; then
  build_release
  exec "$ROOT/apps/desktop/build/linux/x64/release/bundle/koncet" "${PASSTHROUGH[@]}"
fi

echo "Starting KONCET in development mode…"
cd "$ROOT/apps/desktop"
exec "$FLUTTER" run -d linux "${PASSTHROUGH[@]}"
