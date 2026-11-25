# Changelog

All notable changes to KONCET are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) with a single product
version across the monorepo.

## [0.1.0] — Not released

### Added

- Deterministic simulation core: seeded virtual time, build stages, failure injection,
  immutable image manifests, container/process lifecycle, restart backoff, proxy/firewall
  probes, health checks, metrics, logs, and recorded commands.
- SQLite storage with embedded migrations, WAL, pre-migration backups, retention, query-only
  readers, single-instance locking, corruption checks, and interrupted-run reconciliation.
- Application layer for projects, virtual source trees, templates, scenario presets and
  optimistic revisions, one active run session, persisted event/log/metric fan-out, history,
  settings, backups, and diagnostics.
- `flutter_rust_bridge` v2 surface with stable DTOs, typed errors, session/log/metric streams,
  panic containment, and a 100 ms ticker that persists before publishing.
- Flutter Linux desktop app: Material 3 design system, bundled Inter/JetBrains Mono, project
  flow, source editor with autosave, scenario editor, live run workspace, history, settings,
  recovery screen, quit confirmation, and window geometry persistence.
- Version, codegen, test, release, packaging, checksum, notices, and SBOM tooling (`VERSION`,
  `justfile`, `scripts/`).
- Local release artifacts verified: AppImage, `.tar.gz`, `SHA256SUMS`, CycloneDX SBOM, GPL and
  font licence texts, third-party notices.

### Documentation

- Initial product requirements, use cases, and roadmap.
- Architecture overview, simulation-engine design, domain model, data model,
  desktop FFI design, and security model.
- Material 3 design system and screen/flow specification.
- Monorepo, development workflow, testing, and release planning.
- Scenario schema and event catalogue references.
- GPL-3.0-or-later licensing and repository governance documents.

### Known issues — v0.1.0

- GitHub tag and release are not created in this workspace; publish from a clean tagged commit.
- Distribution QA on the full Linux matrix and the offline syscall proof remain release gates.
- Local artifacts were built on a newer baseline than Ubuntu 22.04; the release job must rebuild
  from the tagged commit on the documented Ubuntu 22.04 environment before publishing.
- The in-app scenario editor covers the main fields plus a canonical JSON preview; it does not
  yet expose a raw JSON editing mode.
