# ADR-0008: GitHub-Only Distribution

> **Status:** Accepted · **Date:** 2026-11-24

## Context

KONCET targets Linux desktop users and is maintained by a small team. Distributing through
multiple stores (Flathub, Snap Store, distro repositories) creates separate review, signing,
and update channels without a user base that justifies them. The project must be installable
from one trustworthy location, with checksums and visible release notes, and must not introduce
an auto-updater that conflicts with the offline runtime.

## Decision

Distribute **only through GitHub Releases**, as:

1. an **AppImage** (`KONCET-<version>-linux-x86_64.AppImage`) — primary;
2. a **`.tar.gz`** bundle — fallback for systems where FUSE/AppImage is unavailable;
3. `SHA256SUMS`;
4. release notes and a link to the corresponding source tag;
5. generated third-party notices and SBOM as release assets.

Build on Ubuntu 22.04 to keep the glibc baseline at 2.35. Do not publish to package repositories
or app stores in `v0.1`. Do not implement automatic updates; users download new versions
manually. Data compatibility is preserved through migrations and documented rollback/backup.

## Consequences

**Positive:** one release channel; no store accounts or external reviews; transparent tags and
artifacts; checksums and provenance; compatible with offline runtime; minimal maintenance.

**Negative:** manual discovery and updates; AppImage/FUSE and glibc compatibility issues; no
distro-native integration; GitHub availability and trust are dependencies; users must verify
checksums themselves.

**Mitigations:** clear installation instructions, AppImage extraction fallback, tested
distributions, checksums/GitHub attestations, release notes with known issues, and a future ADR
if a second channel becomes justified.

## Alternatives considered

- **Distro repositories:** high packaging burden across distributions, slow release cadence.
- **Flathub:** good sandboxing but adds store review, runtime maintenance, and advisory model.
- **Snap Store:** vendor-specific infrastructure and confinement complexity.
- **Project website hosting:** duplicates GitHub, adds domain/hosting costs and trust surface.
- **Auto-update service:** violates the offline principle and adds a background network component.
