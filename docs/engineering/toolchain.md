# Toolchain Versions

> **Status:** Verified locally · **Applies to:** `v0.2.0`
> **Last updated:** 2026-11-24
> **Related:** [Development workflow](development-workflow.md) · [Monorepo](monorepo.md)

## 1. Verified versions

| Tool | Version used for `v0.2.0` verification |
| --- | --- |
| Flutter | 3.50.1 stable |
| Dart | 3.14.0 |
| Rust | 1.100.0 (edition 2021, MSRV 1.85) |
| Cargo | 1.100.0 |
| `flutter_rust_bridge` / codegen | 2.13.0 |
| SQLite | bundled `rusqlite` 0.37 / libsqlite3-sys |
| CMake | 4.5.1 |
| Ninja | available in the verified environment |
| Clang | 23 |
| GTK | GTK 3 development headers |
| Linux | x86_64, glibc 2.35+ target baseline |

Flutter and Dart versions travel together. Rust crates are pinned by `Cargo.lock`; Dart
packages by each package's `pubspec.lock`. Generated bridge code is tied to the
`flutter_rust_bridge` codegen version 2.13.0; regenerate with the same version.

## 2. Required system packages (Linux)

- `clang`, `cmake`, `ninja`, `pkg-config`
- `libgtk-3-dev` (or distribution equivalent)
- `libx11-dev`, `libxi-dev` for the window integration plugin
- `libsqlite3` is not required at runtime because SQLite is bundled by `rusqlite`

## 3. Reproduce the checks

```bash
./run.sh --check      # verify Rust, Flutter, native headers, GTK 3
./run.sh              # run from a checkout
./run.sh --install    # build and install as ~/.local/bin/koncet

just check            # maintainer quality gates
just e2e              # Linux integration journey
just build-release
just package
```

`just e2e` needs a working X11 or Wayland session (or `xvfb-run` in CI). The AppImage build
also needs `curl` to fetch `appimagetool` on first use unless `APPIMAGETOOL` points to a local
binary.

## 4. Compatibility policy

- Minimum supported Rust version: 1.85.
- Minimum supported glibc for release artifacts: 2.35.
- Flutter Linux desktop only; no web, mobile, macOS, or Windows artifacts in `v0.1`.
- Advancing Flutter or Rust major/minor toolchains requires a full `just check` and `just e2e`
  run, a regenerated bridge when FRB changes, and release-note documentation.
