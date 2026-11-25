# ADR-0005: Deterministic, Virtual-Time, Event-Driven Simulation

> **Status:** Accepted · **Date:** 2026-11-24

## Context

The core product promise is repeatable learning: the same inputs should produce the same
deployment, failures, logs, and metrics. Real wall-clock timing, thread scheduling, unseeded
randomness, and UI interaction order would make this impossible. The simulator must also run
quickly in tests and support future replay. Real container execution is explicitly out of
scope (ADR-0006).

## Decision

Model simulation as a **single-threaded, event-driven state machine with a virtual clock**.

- Fixed 100 ms tick, paused/accelerated without changing tick size.
- One mutable engine thread and one ordered output stream per run.
- Seeded ChaCha8 streams separated by subsystem; no OS entropy or hash-order dependence.
- User actions are time-stamped commands recorded in a command trace.
- Domain events, logs, and metrics are canonicalised; golden tests compare exact output.
- Randomness, faults, and timings are declarative scenario inputs.
- Observable behaviour changes require an engine version bump and golden updates.
- The engine core is pure: no I/O, wall clock, threads, network, or filesystem.

## Consequences

**Positive:** trustworthy reproducibility; fast headless tests; explainable state; future
replay/compare support; clear separation from UI; easy property-based testing.

**Negative:** less “live/emergent” than fully concurrent simulation; fixed tick introduces
quantisation; interactive runs require a command trace to reproduce; engine changes can
invalidate golden outputs.

**Mitigations:** document the reproduction contract precisely; test canonical serialisation;
keep commands small and typed; store enough metadata for historical inspection; treat golden
updates as intentional changes.

## Alternatives considered

- **Real-time engine with sleeps:** slow tests, scheduler-dependent outcomes, poor reproducibility.
- **Unseeded procedural simulation:** visually interesting but undermines the learning value.
- **Real Docker execution:** violates safety, offline operation, packaging, and product scope.
- **Parallel subsystem threads:** realistic concurrency but nondeterministic ordering and much
  higher complexity for negligible benefit.
- **Record-and-replay UI video:** fragile, storage-heavy, and does not preserve inspectable state.
