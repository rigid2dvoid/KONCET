# `scripts`

Repository automation only; never shipped as part of the desktop runtime.

| Script | Purpose |
| --- | --- |
| `check-version.sh [tag]` | Verify `VERSION`, Cargo, both Flutter manifests, and an optional release tag agree |
| `sync-version.sh VERSION` | Propagate a product version to all manifests and refresh `Cargo.lock` |
| `codegen.sh` | Regenerate the committed `flutter_rust_bridge` bindings and format outputs |
| `build-release.sh` | Verify versions and build the Flutter Linux release bundle (including the Rust cdylib) |
| `package-linux.sh` | Assemble AppImage and `.tar.gz`, copy licences/notices, generate the SBOM and `SHA256SUMS` |
| `generate-sbom.py` | Emit a CycloneDX 1.5 dependency inventory from `Cargo.lock` and `pubspec.lock` |

Scripts are non-interactive and fail on error. The packaging helper downloads `appimagetool`
only when it is not already present; set `APPIMAGETOOL` to a local binary for offline builds.

The repository root also contains two user-facing helpers:

| Helper | Purpose |
| --- | --- |
| `run.sh` | Check the toolchain, fetch dependencies, and launch from a checkout without `just` |
| `install.sh` | Install an AppImage/`.tar.gz` or built bundle under `~/.local`, with desktop entry, version reporting, optional latest-release download, and uninstall |
