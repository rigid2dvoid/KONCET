# KONCET — Product Requirements Document

> **Status:** Baseline · **Applies to:** `v0.1.0`  
> **Last updated:** 2026-11-24  
> **Related:** [Use cases](use-cases.md) · [Roadmap](roadmap.md) · [Architecture](../architecture/overview.md)

## 1. Summary

KONCET is a Linux desktop application that simulates a complete single-node deployment
workflow without connecting to real infrastructure. A user creates a project, edits a
virtual source tree, configures a deployment scenario, and runs it. The simulator
produces a deterministic build, a simulated container image, a container lifecycle, a
deployment status, logs, and basic monitoring data.

The `v0.1.0` release is a small but mature product: one clear loop, fully offline, with
polished Material 3 UI, reliable local persistence, and reproducible results. It is not
a CI/CD product, a real container manager, or a cloud deployment tool.

**Product promise:** *Understand a deployment from source to running service — safely,
visibly, and repeatably.*

## 2. Problem and opportunity

Learning deployment usually requires several disconnected tools and a live environment.
Beginners face expensive mistakes, hidden state, and non-reproducible failures. Existing
simulators are often games, narrow demos, or tied to one vendor. There is room for a
focused desktop workbench that makes the deployment lifecycle tangible while remaining
honest that it is simulated.

KONCET addresses three needs:

1. **Safe practice.** Experiment with builds, container lifecycle, routing, and firewall
   rules without touching a host or cloud account.
2. **Reproducibility.** Re-run the same scenario and seed to study the same sequence of
   events, including failures.
3. **A defensible portfolio product.** Demonstrate product thinking, systems design,
   native desktop engineering, and disciplined release engineering in one GPL project.

## 3. Target users

| Persona | Context | Primary need |
| --- | --- | --- |
| **Learner** — self-taught developer or student | Has read about containers and deployment but has not operated them | A safe, guided environment that exposes the whole loop |
| **Instructor / workshop lead** | Needs repeatable demonstrations and exercises | Deterministic scenarios with known outcomes |
| **Reviewer / maintainer** | Evaluates the project or contributes to it | Clear product scope, documented decisions, stable local data |

KONCET is built for a single local user. Accounts, roles, sharing, and collaboration are
out of scope. The primary interaction language is English; the architecture must not
preclude later localisation.

## 4. Product principles

1. **Simulation, never impersonation.** The UI always says what is simulated. No host
   mutation, no network calls, no hidden real execution.
2. **Determinism over spectacle.** A run is driven by virtual time, a seed, and recorded
   commands. Reproducibility is a correctness property.
3. **One coherent loop.** Project → source → scenario → run → inspect → learn → repeat.
   Depth within the loop beats breadth across DevOps.
4. **Calm density.** The interface is a professional instrument: compact, legible,
   keyboard-friendly, and never cyberpunk or decorative at the expense of clarity.
5. **Local-first ownership.** All user data lives in one SQLite database on the device.
   No account, telemetry, cloud, or background network activity.
6. **Mature smallness.** Every shipped feature has empty, loading, error, and accessibility
   states. Unfinished surfaces do not appear in navigation.

## 5. Goals and non-goals

### Goals for `v0.1.0`

- G-01: Complete the core loop for a single-service deployment.
- G-02: Make every run deterministic and attributable to inputs (scenario, source
  snapshot, seed, command trace, app/engine version).
- G-03: Persist projects, scenarios, runs, events, logs, metrics, and settings reliably.
- G-04: Provide an excellent desktop UX for Linux with a consistent Material 3 system.
- G-05: Ship a reproducible Linux artifact via GitHub Releases under GPL-3.0-or-later.
- G-06: Establish documentation, ADRs, tests, and release gates that support years of
  incremental development.

### Explicit non-goals for `v0.1.0`

- Real Docker/Podman/containerd integration or a real image registry.
- Real Nginx/Traefik/systemd/nftables configuration or host networking.
- Executing source code, real compilers, real dependency downloads, or security scans.
- Kubernetes, multi-node clusters, cloud providers, remote Git, CI/CD integration.
- User accounts, collaboration, sync, telemetry, crash reporting, auto-update.
- Windows and macOS support. The architecture should not block them, but Linux is the
  only supported target for `v0.1.0`.
- Multiple simultaneous active runs or multi-service topologies. The domain model may
  anticipate them; the released UI is intentionally one active run and one service.
- Import/export beyond database backup. Portable project bundles are a later feature.

The permanent boundary is the simulation itself: **no external application or service is
part of the product's runtime**. A future feature may export a report for the user to
read elsewhere, but KONCET will not orchestrate real infrastructure.

## 6. Core concepts

| Term | Meaning |
| --- | --- |
| **Project** | A container for a virtual source tree, scenarios, and run history. |
| **Source snapshot** | Immutable copy of the virtual project files captured when a run starts. |
| **Scenario** | Versioned configuration describing build profile, runtime, networking, and injected faults. |
| **Run** | One simulation session bound to a scenario snapshot and source snapshot. |
| **Virtual time** | Simulation clock that advances in fixed steps and can be paused or accelerated. |
| **Simulated engine** | The four cooperating models: container engine, process manager, reverse proxy, firewall. |
| **Deployment** | The simulated service exposed by a run, with its own status and health. |
| **Command trace** | Ordered record of user actions that affect a run, used for reproducibility. |

See the [glossary](../reference/glossary.md) for the full vocabulary.

## 7. Primary journeys

1. **First deployment.** Create a project from a template, review its files, run the
   baseline scenario, watch the build, inspect the image, observe the container become
   healthy, and open its logs and metrics.
2. **Diagnose a failure.** Run a failure preset, find the failing stage in the build log,
   adjust a file or scenario value, rerun, and compare the outcome.
3. **Repair connectivity.** Run a scenario with a restrictive firewall rule or an invalid
   proxy route, observe the deployment become `DEGRADED`, inspect the denial event, correct the
   scenario, and start a new run that reaches `HEALTHY`.
4. **Review history.** Open an earlier run, filter logs, inspect final container state,
   and confirm the exact scenario/source/seed that produced the result.

Detailed flows and acceptance criteria are in [Use cases](use-cases.md) and
[Screens and flows](../design/screens-and-flows.md).

## 8. Functional requirements

Priorities: **M** = must ship in `v0.1.0`; **S** = should ship if quality allows; **C** = later.
Identifiers are stable and referenced by the phase plan and tests.

### 8.1 Project management

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-PROJ-01 | M | Create a project from at least three templates: Blank, Static Web, HTTP Service. |
| FR-PROJ-02 | M | List, search, open, rename, archive, unarchive, and delete projects. Deletion requires confirmation and removes all owned data. |
| FR-PROJ-03 | M | Show per-project metadata: name, description, template, last edited, latest run result. |
| FR-PROJ-04 | S | Duplicate a project including source, scenarios, but excluding run history. |
| FR-PROJ-05 | M | Prevent empty or whitespace-only names; enforce length limits and display user-facing validation. |

### 8.2 Source code simulation

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-SRC-01 | M | Provide a virtual file tree with create, rename, delete, and open actions; directories are derived from paths. |
| FR-SRC-02 | M | Edit text files with syntax highlighting, line numbers, find, undo/redo, and autosave. |
| FR-SRC-03 | M | Validate paths, file count, file size, and required entrypoint before a run; errors link to the offending file/field. |
| FR-SRC-04 | M | Capture an immutable source snapshot at run start; later edits do not alter the run. |
| FR-SRC-05 | M | Never execute, compile, upload, or open source content outside the virtual editor. |
| FR-SRC-06 | S | Show a clear read-only state and explanation for a file that exceeds the supported size. |
| FR-SRC-07 | C | Import/export a project bundle as a portable archive. |

### 8.3 Scenario authoring

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-SCN-01 | M | Create, edit, duplicate, rename, and delete scenarios within a project. |
| FR-SCN-02 | M | Configure build profile, build durations, service resources, restart policy, health check, proxy route, firewall policy, and supported fault injections through forms. |
| FR-SCN-03 | M | Include four presets: Happy Path, Build Failure, Blocked Port, Crash Loop. |
| FR-SCN-04 | M | Validate configuration inline before running; invalid scenarios cannot start. |
| FR-SCN-05 | S | Offer a read-only JSON view of the effective scenario for advanced inspection. |
| FR-SCN-06 | M | Persist a full scenario snapshot with each run, including its `schema_version`. |
| FR-SCN-07 | C | Import/export scenarios independently of projects. |

### 8.4 Runs and determinism

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-RUN-01 | M | Start a run only when no other run is active. |
| FR-RUN-02 | M | Support pause, resume, stop, and speeds 1×, 2×, 4×, and 8×. |
| FR-RUN-03 | M | Record seed, source snapshot, scenario snapshot, command trace, app version, engine version, and final result. |
| FR-RUN-04 | M | Re-running identical inputs and a scripted command trace must produce an identical ordered event stream for the same engine version. |
| FR-RUN-05 | M | Keep the active run alive while navigating within the app; show an active-run indicator. On quit, warn and require an explicit choice to stop the run and quit or return to the app. |
| FR-RUN-06 | M | On startup, mark any run left active by an abnormal exit as `cancelled` with reason `app_terminated`. |
| FR-RUN-07 | M | Persist a human-readable run summary and a final result independent of live rendering. |
| FR-RUN-08 | S | Provide "Run again" with the same scenario, source snapshot, and a new seed or the same seed. |

### 8.5 Build and image

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-BLD-01 | M | Simulate four ordered stages: Validate, Prepare, Build, Package. Each has deterministic duration and status. |
| FR-BLD-02 | M | Produce streamed, structured build logs with stage context. |
| FR-BLD-03 | M | Support deterministic failure injection per stage and a user-readable failure message. |
| FR-BLD-04 | M | On success, create an immutable simulated image with repository, tag, digest, layers, and size. |
| FR-BLD-05 | M | On failure, produce no image and leave the deployment in `FAILED` / `BUILD_FAILED`. |
| FR-BLD-06 | M | Allow inspection of stages, image layers, digest, and the source snapshot that produced the image. |
| FR-BLD-07 | C | Run multiple builds and keep a project-level image library across runs. |

### 8.6 Container lifecycle and deployment

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-CNT-01 | M | Deploy the built image by creating a simulated container and starting the supervised app process. |
| FR-CNT-02 | M | Implement explicit states and legal transitions: created, starting, running, stopping, stopped, failed, removed. |
| FR-CNT-03 | M | Support stop, start, and restart commands with observable events and logs. |
| FR-CNT-04 | M | Apply CPU/memory limits, environment variables, exposed port, and health-check settings from the scenario. |
| FR-CNT-05 | M | Apply `no`, `on-failure`, and `always` restart policies with bounded backoff and a visible restart count. |
| FR-CNT-06 | M | Detect simulated port conflicts and invalid resource values before deployment. |
| FR-DEP-01 | M | Maintain deployment status: pending, deploying, healthy, degraded, failed, stopped. |
| FR-DEP-02 | M | Derive deployment health from container health, proxy reachability, and firewall outcomes; show a primary reason. |
| FR-DEP-03 | M | Show uptime in simulated time and the last status change. |

### 8.7 Simulated networking

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-NET-01 | M | Configure one reverse-proxy listener and one host/path route to the service port. |
| FR-NET-02 | M | Resolve hosts locally with the `.koncet.local` suffix; never perform real DNS, socket bind, or HTTP requests. |
| FR-NET-03 | M | Configure inbound firewall allow/deny rules, source alias, protocol, and port; evaluate deterministically. |
| FR-NET-04 | M | Surface blocked or misrouted connectivity as proxy logs/events and a degraded deployment. |
| FR-NET-05 | M | Validate duplicate hosts/routes, missing targets, unexposed ports, and contradictory rules. |
| FR-NET-06 | S | Provide an explicit "run connectivity check" action that returns a deterministic result and log entry. |

### 8.8 Logs, observability, and history

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-OBS-01 | M | Display logs from build, engine, container/process, proxy, firewall, and deployment sources with level, source, and simulated timestamp. |
| FR-OBS-02 | M | Filter by source and level, substring search, follow-tail toggle, copy selection, and wrap toggle. |
| FR-OBS-03 | M | Display an ordered event timeline with severity and links to relevant tabs. |
| FR-OBS-04 | M | Persist basic metrics at a configurable simulated interval: container CPU, memory, network rates, restarts, and deployment health. |
| FR-OBS-05 | M | Render live charts for the active run and static charts for historical runs. |
| FR-OBS-06 | M | Show an explicit truncation notice if the per-run log cap is reached. |
| FR-HIS-01 | M | List run history with result, scenario, project, duration, seed, and time; filter and open read-only detail. |
| FR-HIS-02 | M | Deleting a project deletes its runs; deleting a run deletes its events, logs, and metrics after confirmation. |
| FR-HIS-03 | C | Compare two runs or replay a historical command trace step by step. |

### 8.9 Application and settings

| ID | Priority | Requirement |
| --- | --- | --- |
| FR-SYS-01 | M | Provide System/Light/Dark appearance, default speed, and backup settings. |
| FR-SYS-02 | M | Persist window state, active project, and preferences locally. |
| FR-SYS-03 | M | Enforce a single running instance to protect the database. |
| FR-SYS-04 | M | Perform graceful shutdown and SQLite checkpoint; recover cleanly after a crash. |
| FR-SYS-05 | M | Display app version, engine version, database schema version, licence, third-party notices, and diagnostics path. |
| FR-SYS-06 | M | Provide a manual database backup and an automatic pre-migration backup. |
| FR-SYS-07 | M | Run fully offline and expose no telemetry, update check, or remote content. |
| FR-SYS-08 | S | Make all user-facing strings localisation-ready; ship English only in `v0.1.0`. |

## 9. Non-functional requirements

| ID | Requirement |
| --- | --- |
| NFR-PERF-01 | Cold start to interactive Projects screen ≤ 2.5 s on reference hardware; no blocking FFI call > 16 ms on the UI isolate. |
| NFR-PERF-02 | UI animation at 60 fps; scrolling 25,000 log lines remains responsive with virtualisation. |
| NFR-PERF-03 | Engine supports at least 2,000 simulation ticks/s headlessly; 1 simulated hour completes in under 3 s in tests at 100 ms ticks. |
| NFR-PERF-04 | Steady-state idle CPU < 2%; typical memory < 400 MB; large run memory < 700 MB. |
| NFR-SCALE-01 | Support 100 projects, 1,000 stored runs, 25,000 log lines per run, and 1,000 metric points per series without degraded navigation. |
| NFR-REL-01 | All committed data survives normal shutdown and recoverable crashes; SQLite remains consistent. |
| NFR-REL-02 | Migrations are transactional, backed up before execution, and covered by fixture tests across every supported schema version. |
| NFR-DET-01 | For the same engine version and recorded inputs, event ordering, logs, metrics, and final state are byte-equivalent under canonical serialisation. |
| NFR-SEC-01 | The runtime executes no user source, spawns no host processes, binds no sockets, and writes only inside its XDG application directories. |
| NFR-SEC-02 | Database files are user-private (directory `0700`, file `0600`); SQL is parameterised; panics never cross the FFI boundary. |
| NFR-UX-01 | Every asynchronous surface has loading, empty, error, and retry states; destructive actions are confirmed. |
| NFR-UX-02 | WCAG 2.1 AA colour contrast, visible focus, keyboard navigation, semantic labels, 200% text scaling, and reduced-motion support. |
| NFR-UX-03 | Minimum window 1024×720; reference 1440×900; layouts remain usable with 100–200% display scaling. |
| NFR-MAINT-01 | Domain and engine crates are I/O-free; dependencies point in one direction; architecture tests enforce the boundary. |
| NFR-MAINT-02 | Critical domain/engine logic has ≥ 90% branch coverage; workspace overall ≥ 80% line coverage. Coverage is a signal, not a goal. |
| NFR-DIST-01 | Linux x86_64 AppImage and tar.gz are reproducible from a tagged commit on the documented runner; checksums published with each release. |
| NFR-LIC-01 | All distributed code is GPL-3.0-or-later compatible; third-party notices ship with the app. |
| NFR-I18N-01 | No user-facing string is hard-coded in widgets; dates, numbers, and pluralisation use locale-aware formatting. |

## 10. Success metrics for `v0.1.0`

These are product-quality gates, not growth targets. There is no telemetry; evidence
comes from tests, release QA, and structured feedback.

- A new user can complete the baseline deployment without external instructions.
- All four scenario presets produce their documented outcomes across 100 consecutive runs.
- A completed run can be reopened after restart and shows identical final state, logs,
  metrics, and summary.
- Zero network connections observed during a full smoke run under `strace`.
- No P0/P1 defects open at release; all release-gate tests pass on supported distributions.
- At least one external reviewer can build and run the AppImage from the release notes.

## 11. Constraints and assumptions

- Single developer initially; architecture and process must stay proportionate.
- Linux x86_64 and glibc 2.35+ are the only supported baseline for `v0.1.0`.
- Flutter stable and Rust stable are pinned; upgrades are deliberate changes with tests.
- SQLite is sufficient for local, single-user data; no external database is permitted.
- The product is distributed only through GitHub Releases; no store or package repository.
- The app's runtime never depends on network availability.
- AI-generated or third-party assets must be licence-checked and recorded.

## 12. Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Scope expands into a real DevOps platform | Missed release, diluted product | Non-goals are normative; simulation-boundary review on every feature; future ideas go to roadmap, not `v0.1`. |
| Flutter↔Rust bridge blocks or complicates delivery | Schedule and performance risk | Walking skeleton in Milestone 1; thin generated API; no business logic in Flutter. |
| Determinism drifts | Product promise fails | Virtual clock, seeded streams, golden runs, canonical serialisation, engine versioning. |
| SQLite growth or corruption | Loss of history, slow app | Per-run caps, explicit deletion, WAL, pre-migration backups, migration tests, storage screen. |
| Code-editor package is immature | Poor source experience | M1 spike with a tested MIT-licensed native editor; bounded fallback to a simpler editor. |
| Linux packaging incompatibility | Users cannot launch | Build on Ubuntu 22.04 baseline, test matrix, AppImage fallback instructions, checksums. |
| GPL-incompatible dependency slips in | Legal/distribution risk | `cargo-deny`, Dart licence audit, dependency policy, generated notices at release. |
| UI becomes cluttered as statuses grow | Reduced comprehension | Single design system, status-to-visual mapping, progressive disclosure, design review gate. |
| Solo maintainer burnout | Stalled project | Milestone-based plan, documentation, ADRs, tests that protect past work, no artificial deadlines. |

## 13. Release criteria

`v0.1.0` is releasable only when:

- every **M** requirement is implemented and verified;
- all four presets and golden runs pass in CI and locally;
- migration from the initial schema is tested and a pre-migration backup is proven;
- accessibility, performance, offline, and packaging gates in the
  [phase plan](../project/phase-1-plan.md) pass;
- documentation matches shipped behaviour, including screenshots and release notes;
- licence and third-party notices are included in the artifact;
- an AppImage and `.tar.gz` are attached to a GitHub Release with `SHA256SUMS`.

## 14. Deferred decisions

| Question | Default for `v0.1.0` | Revisit |
| --- | --- | --- |
| Code-editor Flutter package | Select in M1 spike; requirements in design docs | M1 |
| Multi-service topology | One service; model supports future expansion | Post-`v0.1` |
| Localisation | English UI, i18n-ready architecture | `v0.2` |
| Auto-update | None | Not planned; conflicts with offline principle |
| Portable project export | Database backup only | `v0.2` |
| Run replay UI | Read-only history only | `v0.2` |
