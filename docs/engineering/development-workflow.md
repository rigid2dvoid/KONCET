# Development Workflow and Standards

> **Status:** Baseline · **Applies to:** implementation phase onward  
> **Last updated:** 2026-11-24  
> **Related:** [Monorepo](monorepo.md) · [Testing](testing-strategy.md) · [Contributing](../../CONTRIBUTING.md)

## 1. Toolchain

Exact versions are pinned at Milestone 1 and updated deliberately.

| Tool | Purpose | Pin |
| --- | --- | --- |
| Flutter stable | Desktop UI | `.fvmrc` or documented version + CI runner |
| Dart | UI language | Bundled with Flutter |
| Rust stable | Core crates | `rust-toolchain.toml` |
| `flutter_rust_bridge_codegen` | Bindings | Exact version in tooling docs/CI |
| `just` | Task recipes | Documented minimum |
| SQLite | Bundled through `rusqlite` | Crate-pinned |
| Linux build deps | Toolchain | `clang`, `cmake`, `ninja`, `pkg-config`, GTK3 dev, `liblzma-dev` |
| `cargo-deny`, `cargo-llvm-cov` | Supply chain and coverage | Pinned in CI/tooling docs |
| `xvfb` | Headless desktop test support | CI image |

A new contributor must be able to go from a clean Ubuntu 22.04/24.04 machine to a running
debug build using the README, `just`, and documented packages only.

## 2. Bootstrap sequence

1. Install Flutter and Rust per pinned toolchain files.
2. Install Linux packages from the documented list.
3. `./run.sh --check` — verify Rust, Flutter, native headers, and GTK 3.
4. `./run.sh` — fetch Dart dependencies and launch the Flutter desktop app.
5. `./run.sh --install` — build release and install as `~/.local/bin/koncet`.
6. `just check` — format check, lint, Rust tests, Dart analyse/tests.
7. `just e2e` — run the Linux integration journey on a display.
8. `just codegen` — regenerate the committed bridge after changing the FFI surface.
9. `just build-release` / `just package` — produce and verify release artifacts.

The documented toolchain for this tree is in [Toolchain versions](toolchain.md). Generated
bridge code is committed, so a fresh checkout can build without running codegen.

## 3. Branching and pull requests

- Trunk-based development: `main` is the integration branch.
- Short-lived branches: `feat/...`, `fix/...`, `docs/...`, `chore/...`.
- Pull requests should be small enough to review in one sitting and include:
  - problem and approach;
  - tests and documentation updates;
  - screenshots/recordings for visible changes;
  - migration and rollback notes for data changes;
  - licence impact for new dependencies.
- Required before merge once CI exists: format, lint, tests, build, licence check, docs links.
- A review is expected for engine, schema, security, or release changes. A solo maintainer must
  still run the full checklist and record self-review evidence in the PR.
- Never commit directly to `main` after the initial bootstrap, except for an emergency that is
  documented in the PR/commit and followed by a normal review.

## 4. Commit conventions

Use Conventional Commit prefixes so history is readable and automation can classify changes:

```text
feat(sim): add restart backoff
fix(storage): roll back failed migration
docs(adr): record offline simulation boundary
test(ui): add dark-mode log viewer golden
refactor(ffi): map domain errors centrally
chore(release): prepare v0.1.0
```

Rules:

- Subject in imperative mood, ≤ 72 characters, no trailing period.
- Body explains *why* and any user-visible consequence.
- Reference issues (`Refs: #123`) where useful.
- Do not use “misc”, “updates”, or “fix stuff”.
- Commits must not contain secrets, local databases, generated build output, or machine paths.

## 5. Rust standards

- `cargo fmt` and `cargo clippy --workspace --all-targets -- -D warnings` are mandatory.
- Prefer typed errors (`thiserror` or domain enums) over stringly errors.
- No `unwrap`/`expect` in production paths unless the invariant is proven and documented; tests
  may use them.
- No `unsafe` outside generated FFI glue; any exception requires an ADR and focused review.
- No I/O, threads, wall clock, OS RNG, or environment access in `koncet-domain`/`koncet-sim`.
- Public APIs are documented; internal visibility defaults to private.
- Avoid unchecked integer casts; validate bounds at DTO boundaries.
- Keep engine state ordered and canonical; do not leak hash iteration order into output.
- Use `tracing` for diagnostics; simulation logs go through the engine log model.
- Deny network/process APIs in shipping crates via lint/architecture tests.
- New dependencies require justification in the PR and pass licence review.

## 6. Dart/Flutter standards

- `dart format` and `flutter analyze` clean; no ignored warnings without an inline justification.
- Riverpod for dependency injection and async/stream state; `go_router` for routes.
- Widgets are small, const where possible, and contain no business/validation logic.
- Use design tokens/components from `packages/koncet_ui`; no hard-coded colours/spacings.
- All user-visible strings go through localization keys (`v0.1` English catalogue).
- Async UI handles loading/empty/error/retry; avoid fire-and-forget calls.
- Use semantics labels for icon-only controls and charts.
- No network packages or remote asset loading.
- Prefer immutable DTOs generated by the bridge; do not maintain duplicate domain models.
- Rebuild scope: selectors/`select` on providers; avoid rebuilding whole run screens on ticks.

## 7. Testing and review gates

- Every bug fix includes a regression test.
- Every engine rule includes a headless deterministic test.
- Every schema change includes a migration test from each supported version.
- Every screen change includes widget/golden coverage for light and dark.
- Every new dependency includes licence and security review.
- Coverage is measured; CI fails if critical crates drop below the agreed thresholds.
- Performance-sensitive changes require before/after numbers on reference hardware.

See [Testing strategy](testing-strategy.md).

## 8. Documentation standards

- Follow the structure and metadata block used in `docs/`.
- Use relative links and Mermaid for diagrams; no binary-only architecture diagrams.
- Use RFC 2119 terms precisely.
- Keep prose direct and implementable; avoid marketing language in technical documents.
- Update docs and screenshots in the same PR as behaviour changes.
- Add an ADR for cost-of-reversal decisions, not for routine choices.
- Run a Markdown link check in CI; relative links must resolve.
- Record code-of-conduct and translation processes before the first external contributor joins.

## 9. Dependency policy

Allowed licences (GPL-3.0-or-later compatible): MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib,
MPL-2.0 (with care), OFL for fonts, public domain (SQLite). GPL-compatible copyleft is allowed
when obligations are understood.

Rejected or escalated: proprietary, source-available without OSI approval, SSPL, Commons Clause,
unknown licence, GPL-incompatible terms, packages with unclear maintenance or excessive native
surface. Apache-2.0 is compatible with GPLv3; do not add GPLv2-only dependencies.

For each new dependency, record:

- why the standard library or an existing dependency is insufficient;
- maintenance activity and security history;
- transitive licence/native impact;
- whether it opens network, filesystem, process, or unsafe surface;
- removal/replacement plan if abandoned.

Dart packages with native Linux plugins need explicit desktop verification; web/mobile-only
packages are rejected.

## 10. Local data hygiene

- Development uses a dedicated data directory (for example `KONCET_DATA_DIR=/tmp/koncet-dev`).
- Never commit a real database, backup, diagnostic log, or user content.
- Test fixtures are synthetic and contain no personal data.
- Reset local data only with the app closed and a backup when relevant.

## 11. Definition of Done

A change is done when:

- behaviour and acceptance criteria are met;
- tests cover success, failure, and boundary cases;
- format/lint/type checks pass;
- docs, schemas, ADRs, and screenshots are updated where affected;
- accessibility and keyboard paths are verified for UI changes;
- performance budgets are not regressed;
- licence obligations are satisfied;
- no secrets or user data are included;
- the PR explains rollout/migration/rollback where relevant.
