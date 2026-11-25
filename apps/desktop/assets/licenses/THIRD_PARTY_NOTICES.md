# KONCET third-party notices

KONCET itself is licensed under GPL-3.0-or-later. The full text is bundled as
`GPL-3.0-or-later.txt` and in the repository root.

## Fonts

- **Inter** — Copyright The Inter Project Authors. SIL Open Font License 1.1.
  See `OFL-Inter.txt`.
- **JetBrains Mono** — Copyright JetBrains s.r.o. SIL Open Font License 1.1.
  See `OFL-JetBrainsMono.txt`.

## Flutter and Dart

Flutter, Dart, and the bundled Flutter engine are Copyright the Flutter Authors,
licensed under the BSD 3-Clause licence. The Flutter tool includes these notices
in the application's licence registry.

## Rust dependencies

The Rust crate graph is recorded in `Cargo.lock`. All dependencies are
compatible with GPL-3.0-or-later; the release process must regenerate this
notice and a CycloneDX SBOM from the locked dependency graph before publishing.

## Control

KONCET performs no network calls at runtime, sends no telemetry, and bundles no
tracking SDK. Simulated hostnames use the reserved `.koncet.local` suffix.
