# Monorepo Structure and Versioning

> **Status:** Baseline · **Applies to:** all releases  
> **Last updated:** 2026-11-24  
> **Related:** [ADR-0001](../architecture/adr/0001-monorepo-single-version.md) · [Development workflow](development-workflow.md) · [Release](release-and-distribution.md)

## 1. Repository layout

```text
KONCET/
├── apps/
│   └── desktop/                 Flutter Linux desktop application
│       ├── lib/                 presentation, routing, Riverpod providers
│       ├── linux/               native runner
│       ├── test/                widget and unit tests
│       ├── integration_test/    end-to-end desktop tests
│       └── pubspec.yaml
├── packages/
│   └── koncet_ui/               Material 3 design system (Dart package)
├── crates/
│   ├── koncet-domain/           pure entities, value objects, state machines, schemas
│   ├── koncet-sim/              deterministic engine (no I/O)
│   ├── koncet-storage/          SQLite migrations and repositories
│   ├── koncet-app/              use cases, sessions, event fan-out
│   └── koncet-ffi/              flutter_rust_bridge facade and cdylib
├── docs/                        source of truth for product and engineering
├── scripts/                     version sync, codegen, packaging, SBOM
├── Cargo.toml                   Rust workspace root
├── flutter_rust_bridge.yaml     FRB inputs and generated outputs
├── run.sh                       one-command source launcher
├── install.sh                   user-local installer and uninstaller
├── justfile                     developer and release recipes
├── VERSION                      single product version source of truth
├── CHANGELOG.md
├── CONTRIBUTING.md
├── LICENSE
├── README.md
└── SECURITY.md
```

All workspaces above are implemented for `v0.1.0`. `apps/desktop` and `packages/koncet_ui` are
independent Dart packages linked by a path dependency; each keeps its own `pubspec.lock`. The
Flutter release build compiles the `koncet-ffi` cdylib through CMake and bundles it under
`lib/`.

## 2. Dependency rules

```text
apps/desktop ──────────────► packages/koncet_ui
      │
      └── generated Dart bindings ──► crates/koncet-ffi
                                             │
                                             ▼
                                        koncet-app
                                         │      │
                                ┌────────┘      └────────┐
                                ▼                        ▼
                          koncet-sim              koncet-storage
                                │                        │
                                └──────────┬─────────────┘
                                           ▼
                                     koncet-domain
```

Rules:

- `koncet-domain` depends only on foundational utility crates (`serde`, `uuid`, time value
  types). No database, filesystem, network, Flutter, or FFI.
- `koncet-sim` depends on `koncet-domain`; no I/O, wall clock, OS RNG, threads, or storage.
- `koncet-storage` depends on `koncet-domain`; owns all SQL and migration logic.
- `koncet-app` composes sim and storage; contains use cases and session orchestration.
- `koncet-ffi` depends on `koncet-app`; no business logic.
- `apps/desktop` depends on `packages/koncet_ui` and the generated bridge; never on Rust
  internals or SQL.
- `packages/koncet_ui` depends only on Flutter and presentation libraries; no domain or app
  imports.
- No cycles. CI/architecture tests enforce banned dependencies and APIs (`std::process`,
  networking in shipping crates).

## 3. Workspace tooling

- Rust: one Cargo workspace with shared `[workspace.package]` version, edition, licence, and
  dependency versions. Commit `Cargo.lock`.
- Dart: `apps/desktop` depends on `packages/koncet_ui` by path. Commit both
  `pubspec.lock` files. There is no root Dart package.
- Generated bridge sources are committed; `just codegen-check` regenerates and fails on
  drift. CI runs this check; contributors run `just codegen` after changing the FFI surface.
- Local commands are exposed through `just` recipes, which call `cargo`, `flutter`, and
  `scripts/`. CI uses the same recipes where practical.
- No generated files are hand-edited.

## 4. Single product version

- `VERSION` at the repository root contains `MAJOR.MINOR.PATCH` (for example `0.1.0`).
- `scripts/sync-version.sh` updates:
  - Cargo `[workspace.package].version`,
  - `apps/desktop/pubspec.yaml` version and build number,
  - `packages/koncet_ui/pubspec.yaml` version,
  - displayed build metadata if generated.
- `scripts/check-version.sh` asserts every manifest equals `VERSION` and that a release tag
  matches exactly.
- Publication is a **tag**, not a manifest change: `v0.1.0`, `v0.1.1`, `v0.2.0-rc.1`.
- Internal crates and the UI package are never published independently.
- Run metadata records app version, engine version (currently equal to product version),
  source commit, and schema versions.

Version semantics follow SemVer for the product:

- `MAJOR`: incompatible product/data contract changes after 1.0; pre-1.0 may break with
  migration and release notes.
- `MINOR`: new user-facing capability, backwards-compatible data.
- `PATCH`: fixes, documentation, performance, packaging.

## 5. Generated artifacts and lockfiles

| Artifact | Commit? | Reason |
| --- | --- | --- |
| `Cargo.lock` | Yes | Reproducible application builds |
| `pubspec.lock` (workspace root) | Yes | Reproducible desktop builds |
| FRB generated Rust/Dart bindings | Yes | Reviewable API changes; drift checked |
| Flutter platform runner (`linux/`) | Yes | Part of the product build |
| `.dart_tool/`, `target/`, build outputs | No | Disposable |
| Local databases, logs, backups | No | User data; outside repository |
| SBOM, notices, packaged artifacts | No (release assets) | Generated at release |

## 6. Documentation in the monorepo

- `docs/` is versioned with the code and reviewed like code.
- ADRs are immutable once accepted; new decisions supersede old ones.
- Product copy and screenshots must match the tagged release.
- Reference schemas (`reference/`) are normative and require tests when changed.
- The root `README` links only to durable entry points, not deep implementation details.

## 7. Ownership and change scope

- A single change may cross crates and Dart when needed; keeping the contract coherent is more
  important than artificial per-crate isolation.
- Schema changes require a migration, data-model update, backup path, and migration test.
- Engine behaviour changes require golden updates and version notes.
- UI system changes require design-system documentation and golden updates.
- New directories require a documented purpose; do not add speculative structure.
