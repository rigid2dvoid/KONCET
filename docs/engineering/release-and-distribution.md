# Release and Distribution

> **Status:** Baseline · **Applies to:** `v0.1.0` and later  
> **Last updated:** 2026-11-24  
> **Related:** [ADR-0007](../architecture/adr/0007-gpl-3.0-or-later.md) · [ADR-0008](../architecture/adr/0008-github-only-distribution.md) · [Monorepo](monorepo.md)

## 1. Release model

- **One product, one version, one tag.** Tags follow `vMAJOR.MINOR.PATCH` (or `-rc.N`).
- Releases are GitHub Releases only. No app stores, package repositories, or independent
  package publication.
- Pre-1.0 releases are development releases; data compatibility is still protected by migrations.
- A release is generated from an annotated/signed tag on `main` after all gates pass.
- No automatic updater exists. Users download and replace the application manually.

## 2. Versioning

| Change | Version |
| --- | --- |
| Incompatible product contract or data format (with migration) after 1.0 | MAJOR |
| New user-facing capability, backwards-compatible data | MINOR |
| Bug fix, performance, docs, packaging | PATCH |
| Release candidate | `-rc.N`, published as GitHub pre-release |
| Broken release that must be withdrawn | Never re-tag; publish a fixed PATCH and mark notes |

The root `VERSION`, Cargo workspace, Flutter manifests, and tag must match exactly. Pre-release
versions carry the suffix consistently in all manifests.

## 3. Supported platform and artifacts

**Baseline:** Linux x86_64, glibc 2.35+, built on Ubuntu 22.04.

| Asset | Purpose |
| --- | --- |
| `KONCET-<version>-linux-x86_64.AppImage` | Primary portable application |
| `KONCET-<version>-linux-x86_64.tar.gz` | Fallback bundle for FUSE-restricted systems |
| `SHA256SUMS` | Integrity verification |
| `KONCET-<version>-sbom.cdx.json` | Dependency inventory (CycloneDX) |
| `THIRD_PARTY_NOTICES.md` | Licence obligations |
| `KONCET-<version>-source.tar.gz` (optional) | Convenience source snapshot; the git tag is canonical |

Assets include a text/PDF-free licence file and a link to the source tag in release notes.
Windows/macOS are not supported; no artifacts are published for them.

## 4. Release gates

All must pass:

1. `VERSION` equals tag and all manifests.
2. Working tree clean; build from the tagged commit only.
3. Format, lint, Rust/Dart tests, golden runs, migration tests, FFI contract tests.
4. End-to-end smoke journey under Linux.
5. Performance budgets not regressed beyond agreed tolerance.
6. Accessibility/golden review evidence for changed screens.
7. Offline check: no runtime network syscalls in smoke run.
8. `cargo-deny` and Dart licence audit pass; GPL compatibility confirmed.
9. Database backup/migration path tested; rollback notes written.
10. Docs, changelog, screenshots, and known issues updated.
11. AppImage launches on the test matrix (at least Ubuntu 22.04/24.04 and Fedora).
12. Artifact checksums and SBOM generated and verified.
13. No P0/P1 defects open; security advisories triaged.
14. Release notes describe user-visible changes, migration notes, and limitations.

## 5. Build procedure (planned)

```text
1. just check                         # local parity with CI
2. just version-check                 # VERSION == manifests
3. just build-release                 # Flutter Linux release + Rust cdylib
4. just package                       # AppImage + tar.gz + notices + SBOM
5. just verify-artifacts              # sha256, launch smoke, permission audit
6. git tag -s vX.Y.Z && git push --tags
7. CI release job builds from the tag, repeats gates, uploads assets
8. Create GitHub Release with notes, checksums, SBOM, notices; mark pre-release if -rc
```

Packaging rules:

- Strip debug symbols from the shipped binary; keep a separate symbol archive if needed.
- Bundle fonts, icons, GTK/Flutter runtime pieces allowed by their licences.
- Do not bundle personal data, dev databases, test fixtures, or source maps.
- AppImage desktop entry: name KONCET, categories Development/Utility, icon installed at the
  correct sizes.
- `--appimage-extract-and-run` documented for FUSE-less environments.
- Build number metadata includes tag and commit; no dirty builds are released.

## 6. Checksums, signing, and provenance

- Publish `SHA256SUMS` and include verification commands in the release notes.
- Prefer GitHub artifact attestations/build provenance for CI-produced assets.
- A detached GPG signature may be added when a stable maintainer key exists; it is not required
  for `v0.1.0`.
- Users are instructed to download only from the official GitHub Releases page.
- Third-party notices and the GPL source link are mandatory on every release.

## 7. Changelog and release notes

- `CHANGELOG.md` follows Keep a Changelog and is updated in the release PR.
- Release notes start with a short user-facing summary, then sections:
  **Added**, **Changed**, **Fixed**, **Security**, **Known issues**, **Upgrade notes**,
  **Verification**.
- Call out simulation-fidelity changes and deterministic golden updates.
- Link to the diff and the exact source tag.
- Do not promise dates or unsupported platforms.

## 8. Upgrade, migration, and rollback

- Data migrations run automatically at startup after a verified backup.
- Backups are stored under the XDG data directory and the path is shown in Settings.
- An older application refuses a newer database schema; it does not attempt a downgrade.
- Rollback procedure: close the app, restore the pre-migration backup, install the older
  version. This is documented in release notes for schema-changing releases.
- Historical runs remain readable across upgrades; unknown event types render generically.
- Engine changes do not recompute historical runs.

## 9. Distribution policy

- GitHub Releases is the sole official distribution channel. There are no official mirrors,
  package repositories, stores, paid tiers, hosted services, telemetry, or runtime network calls.
- Third-party mirrors are unsupported; users should verify checksums and obtain assets only from
  the project's GitHub Releases page.
- A release may be withdrawn (marked as pre-release/edited notes) if unsafe; a fixed version is
  issued rather than silently replacing bytes.
- Security fixes are released as PATCH versions with an advisory when appropriate.

## 10. Post-release

- Verify downloads and checksums from a clean machine.
- Announce in the repository discussions/release notes only; no tracking links.
- Triage issues with labels: `bug`, `regression`, `docs`, `security`, `needs-repro`.
- Schedule a retrospective for release-gate failures; update this document and automation.
- Keep at least the latest stable release assets available; old releases may be retained for
  reproducibility but are unsupported.

## 11. Release checklist

- [ ] Scope matches the milestone exit criteria.
- [ ] Version synced and tag created from a clean commit.
- [ ] Full automated and manual gates recorded.
- [ ] Changelog, docs, screenshots, known issues updated.
- [ ] AppImage and tar.gz verified on the distribution matrix.
- [ ] SHA256SUMS, SBOM, third-party notices attached.
- [ ] GPL source link and licence present.
- [ ] Database backup/rollback notes verified.
- [ ] No network calls, no secrets, no user data in artifacts.
- [ ] GitHub Release published with correct pre-release flag.
- [ ] Post-download verification completed.
