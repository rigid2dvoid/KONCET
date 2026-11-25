# ADR-0001: Monorepo and One Product Version

> **Status:** Accepted · **Date:** 2026-11-24

## Context

KONCET consists of a Flutter desktop shell, several Rust crates, a shared Material 3 design
system package, documentation, and release tooling. These parts always ship together: a UI
build is meaningless without a matching Rust library, and release notes must describe one
coherent desktop product. The project targets GitHub Releases only and has a single
maintainer, with room for contributors. Multiple repositories would add cross-repo version
coordination without any independent consumers.

## Decision

Use one monorepo with a **single product version** (`MAJOR.MINOR.PATCH`, SemVer).

- Flutter app, Rust crates, UI package, docs, and scripts live together.
- Rust crates inherit the workspace version; Dart manifests use the same product version.
- A root `VERSION` file is the source of truth; a script synchronises manifests and CI
  verifies equality.
- Release tags are `vX.Y.Z`; every build's metadata includes the tag and commit.
- Internal crates/packages are never published independently.
- One `CHANGELOG.md` describes the product, not individual crates.

## Consequences

**Positive:** atomic changes across UI/core/schema; one release gate; simple contributor
onboarding; consistent tags and container-free artifacts; documentation changes ship with code.

**Negative:** CI may build more than needed; Flutter and Rust toolchains coexist; version
synchronisation needs tooling; a change in any component may require re-running the whole
quality gate.

**Mitigations:** path-filtered checks later, `just` recipes for local parity, version-check
script, and a release checklist. The monorepo remains small enough that full release builds
are inexpensive.

## Alternatives considered

- **Polyrepo (UI, core, docs separately):** rejected; no independent consumers, high version
  and release overhead.
- **Monorepo with independently versioned packages:** rejected; violates “one product, one
  version”, adds tooling without benefit.
- **Git submodules:** rejected; poor contributor and release ergonomics.
