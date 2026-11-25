# ADR-0003: Rust Core Embedded via In-Process FFI

> **Status:** Accepted · **Date:** 2026-11-24

## Context

The simulation engine must be fast, deterministic, and safe, and it must persist history
without blocking Flutter's UI. It cannot depend on network services or external daemons.
The UI is built in Flutter (ADR-0002). We need a boundary that is testable, packaged as one
application, and does not introduce a local server or extra process lifecycle.

## Decision

Implement domain, simulation, storage, and use cases in **Rust**, compiled as a `cdylib`
embedded in the Flutter Linux application. Communicate through **`flutter_rust_bridge` v2**
generated bindings using async commands and typed streams.

- One process, no TCP/Unix socket server.
- The Rust side owns the engine thread, DB writer, read pool, and use cases.
- The Dart side owns presentation, navigation, and form state only.
- Generated bridge code is committed; regeneration drift is checked.
- Panics are contained at the FFI boundary and mapped to typed errors.

## Consequences

**Positive:** no port allocation or local attack surface; low IPC latency; atomic packaging;
single lifecycle; Rust performance and memory safety for the engine; straightforward headless
tests of the core.

**Negative:** two toolchains and build systems; generated code churn; debugging crosses
languages; a native crash can take down the UI; FFI API must be designed deliberately.

**Mitigations:** walking-skeleton milestone, thin facade, contract tests, panic isolation,
structured diagnostic ids, deterministic build instructions, and no business logic in Dart.

## Alternatives considered

- **Pure Dart engine:** simpler build but less suitable for a long-lived performance-sensitive
  simulator and future multi-service workloads; still needs database access.
- **Rust sidecar process + local HTTP/gRPC:** clean process isolation but adds ports,
  authentication, startup supervision, packaging, and a network-like surface contrary to the
  offline principle.
- **Hand-written C ABI:** maximum control, high unsafe/maintenance cost, no generated streams.
- **Tauri with Rust backend:** conflicts with the Flutter/Material 3 decision and introduces a
  webview.
