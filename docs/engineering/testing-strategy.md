# Testing Strategy

> **Status:** Baseline · **Applies to:** `v0.2.0`
> **Last updated:** 2026-11-24
> **Related:** [Development workflow](development-workflow.md) · [Simulation engine](../architecture/simulation-engine.md) · [Phase 1 plan](../project/phase-1-plan.md)

## 1. Principles

- Test the **contract**, not implementation details: observable events, state, persistence, and
  UI behaviour.
- The engine must be testable without the UI, display server, wall-clock waiting, or network.
- Prefer deterministic fixtures and golden runs over brittle timing-based tests.
- Every bug fix earns a regression test.
- Coverage is a diagnostic, not a vanity target. Critical state machines and persistence paths
  are held to a higher bar.
- Tests must be runnable locally with the same commands used in CI.

## 2. Test pyramid

| Layer | Scope | Tooling | Runs on |
| --- | --- | --- | --- |
| Unit | Domain invariants, validators, formatting, repository mappings | Rust test, Dart test | Fast, no display |
| Headless simulation | Scenario → canonical events/logs/metrics/final state | Rust test harness, golden JSON | Fast, no display |
| Property | State-machine legality, no panics, bounded data | `proptest` | Fast, no display |
| Storage integration | Migrations, transactions, cascades, corruption handling | Temporary SQLite files | Linux |
| FFI contract | DTO mapping, stream batching, error mapping, panic containment | Rust + Dart integration | Linux |
| Widget/golden | Design system, screen states, light/dark, scaling | Flutter widget/golden tests | Linux, `xvfb` where needed |
| End-to-end | Core journey through the desktop app | Flutter integration_test | Linux CI/display |
| Performance | Engine throughput, log scrolling, startup, DB queries | Benchmarks with budgets | Reference runner |
| Security/offline | Path corpus, oversized inputs, network syscalls, licences | Scripts + `strace`, `cargo-deny` | Release gate |

## 3. Required Rust tests

### Domain

- Constructors reject empty names, invalid paths, bad seeds, resource values, and route rules.
- Every state machine rejects illegal transitions with the correct typed error.
- Deployment-status derivation is table-driven across all subsystem combinations.
- Invariants from [Domain model §7](../architecture/domain-model.md#7-invariants-worth-testing)
  are executable tests.

### Simulation

- Golden run for each preset: Happy Path, Build Failure, Blocked Port, Crash Loop.
- Same inputs run twice → identical canonical event/log/metric output and final snapshot.
- Command trace changes outcome only through recorded commands; replaying the trace reproduces.
- Build failure produces no image/deployment; blocked firewall yields DEGRADED, not HEALTHY.
- Restart backoff, exhaustion, and manual restart follow the documented timeline.
- Log cap emits exactly one truncation event; domain events remain complete.
- Duration limit terminates cleanly and finalises the run.
- No wall-clock, thread, filesystem, or network APIs in `koncet-sim` (architecture test).
- Property tests: event sequence is gapless; clock is monotonic; restart count never decreases;
  no terminal state transitions.

### Storage

- Fresh database applies all migrations and passes `foreign_key_check`.
- Each supported older schema migrates successfully, with backup creation verified.
- Altered migration checksum aborts startup safely.
- Project deletion leaves no orphans in every owned table.
- Run deletion removes events, logs, metrics, builds, images, containers, deployments.
- Concurrent read during write does not fail or block UI-level time budgets.
- Keyset pagination returns stable, non-duplicated results.
- Backup → mutate → restore produces the original state (at least for smoke fixtures).

### FFI/app

- Every domain error has a mapping; an unmapped error fails the contract test.
- A deliberately panicking test endpoint is converted to `internal_error` without unwinding.
- Stream frames are ordered, bounded, and recover from a dropped subscriber.
- Starting a second run returns `active_run_exists` and leaves the first unaffected.
- Large log/metric queries respect page and batch limits.

## 4. Dart/Flutter tests

- Design-system component tests for every shared widget, including disabled/loading/error.
- Status mapping: every enum value has icon + label + tone; no unknown falls through silently.
- Golden tests at 1024×720 and 1440×900, light and dark, for:
  - Projects empty/loading/list;
  - Source editor with validation error;
  - Scenario editor subset;
  - Run overview healthy and build-failed;
  - Runtime crash loop;
  - Network blocked;
  - Logs empty/filtered/truncated;
  - Metrics with data and empty.
- Accessibility checks for semantics, focus order, and minimum target sizes.
- Text scaling 200% and reduced motion variants for critical screens.
- Riverpod controller tests with fake new bridge streams: subscription lifecycle, duplicate frame
  rejection, active-run reconnect, error states.
- No widget test relies on real time; inject fake clocks/streams.

## 5. End-to-end tests

At least these automated journeys on Linux (headless display allowed):

1. Create Static Web project → start Happy Path → run at 8× to completion → assert HEALTHY,
   image present, logs/metrics visible, run appears in history.
2. Build Failure preset → assert failing stage, no image, result `build_failed`, log message.
3. Blocked Port preset → assert DEGRADED, primary reason `firewall_denied`, connectivity check
   fails, fixing the rule in a new run yields HEALTHY.
4. Crash Loop preset → assert restart count reaches limit, container FAILED, reason
   `restart_exhausted`.
5. Restart the app → completed run reopens in read-only mode with the same final data.
6. Quit during active run → prompt path stops cleanly and finalises; forced kill → next launch
   reconciles to `app_terminated`.
7. Second-instance launch is refused or focuses the existing window without opening a second
   database writer.

Tests may use a debug-only accelerated clock or “run to completion” test hook, never production
shortcuts that alter determinism.

## 6. Golden-run format

- Inputs: scenario JSON, source fixture, seed, command trace, engine version.
- Output: canonical JSON files for events, logs, metrics, final snapshot, and summary.
- Keys sorted, floats rounded to fixed precision, timestamps normalised.
- Stored under test fixtures for the owning crate; fixture changes must be explained in review.
- A tool prints a readable diff of the first divergence; raw snapshots alone are insufficient.

## 7. Performance tests

| Test | Budget |
| --- | --- |
| Cold start to interactive Projects (release, warm OS cache) | ≤ 2.5 s |
| Idle CPU with UI open, no run | < 2% |
| Memory, typical project with one completed run | < 400 MB |
| Engine headless: 1 simulated hour at 100 ms ticks | < 3 s |
| UI update handling with active run at 8× | 60 fps target; no frame > 33 ms p95 |
| Log list with 25,000 records | responsive scroll, filter < 300 ms |
| Run history with 1,000 runs | first page < 200 ms p95 |
| Backup of 500 MB database | bounded and progress-visible |

Benchmarks run on documented reference hardware; results are stored in PR notes for regressions.
Micro-benchmarks do not gate merges unless they map to a user-visible budget.

## 8. Security and offline verification

- Path traversal corpus: `..`, absolute paths, NUL, Unicode confusables, overlong paths.
- Oversized source, scenario, env, log, and page-size inputs are rejected or capped.
- SQL injection payloads are stored as literal data and displayed safely.
- `strace -f -e trace=network` over a full smoke run: no application `connect`/`bind`/`listen`.
- `cargo-deny` prohibits incompatible licences, advisories, and banned crates.
- Dart dependency licence audit and native-plugin review.
- File permissions test: data dir `0700`, db/backups `0600`.
- Fuzz scenario deserialisation and FFI DTO boundaries (bounded time in CI).
- Release asset checksums verify against the built files.

## 9. CI stages (implemented after scaffolding)

1. Docs: Markdown links, Mermaid syntax where feasible, version references.
2. Format/lint: `cargo fmt --check`, `cargo clippy -D warnings`, `dart format --output=none`,
   `flutter analyze`.
3. Rust tests: unit, golden, property, storage, FFI contract.
4. Dart tests: unit, widget, golden; headless display if required.
5. Build: Linux debug/release, generated bindings drift check.
6. End-to-end smoke: minimal journey under `xvfb`.
7. Supply chain: `cargo-deny`, Dart licence audit.
8. Release job on tag: full gate + packaging + checksums + SBOM + notices.

Until CI exists, the same `just` recipes are the local gate; no release is made without them.

## 10. Coverage and quality targets

- `koncet-domain` and `koncet-sim`: **≥ 90% branch coverage**, including all state transitions
  and preset paths.
- `koncet-storage` and `koncet-app`: **≥ 85% line coverage**, with migration and deletion paths
  fully covered.
- UI: critical screens and components have golden/state coverage; percentage is not the gate.
- Zero unwaived lints; zero high/critical security advisories; zero denied licences.
- Known skipped tests require an issue, reason, and owner in the PR.

## 11. Manual QA matrix before release

- Ubuntu 22.04, Ubuntu 24.04, Fedora latest, Arch current.
- X11 and Wayland; 100%, 125%, 150%, 200% display scaling.
- Light/dark/system themes; reduced motion; 200% text.
- Install/run via AppImage (FUSE) and AppImage extraction fallback; tar.gz launch.
- Fresh profile, upgraded profile with pre-existing database, read-only data directory failure.
- No network at runtime; no unexpected writes outside XDG dirs.
- Keyboard-only main journey; screen-reader smoke test where tooling permits.

## 10. `v0.2.0` implementation status

Implemented and runnable:

- Rust: 38 domain unit tests, 10 simulation tests (four presets, determinism, sequence
  integrity, pause, limits), 18 storage tests (migrations, round trips, cascades, backup,
  lock, corruption, query-only reader), 13 application tests, 2 bridge error tests, and one
  end-to-end bridge smoke test. Architecture guards scan `koncet-domain` and `koncet-sim`
  source for filesystem/network/process APIs.
- Dart: theme/component tests, formatting/status mapping tests, an active-run controller test
  with fake streams, and a Linux integration journey covering project creation, source editor,
  scenario editor, live run to healthy, stop, history, details, and settings.
- A separate screenshot harness seeds representative healthy, failed, degraded, and crashed
  runs and captures the README showcase from the real Flutter render tree at 1440×900.
- Release: local AppImage and `.tar.gz` built, checksum-verified, and launched; SBOM, notices,
  GPL and font licences included.

Not yet checked in for this release branch:

- Full canonical golden JSON fixtures. Determinism is currently enforced by the engine equality
  test, sequence checks, and preset assertions; fixture files should be added before the public
  tag so review diffs show behavioural changes explicitly.
- `proptest` suites and the performance benchmark harness.
- Golden screenshots at 1024×720 and 1440×900 in both themes; the widget tests assert layout
  and content instead.
- The full Linux distribution matrix, accessibility audit evidence, and offline syscall proof.

These gaps are release gates, not optional polish, and are tracked in the phase-1 plan.
