# Architecture Decision Records

> **Status:** Index · **Last updated:** 2026-11-24

Architecture Decision Records capture decisions that are costly to reverse or that shape
multiple parts of KONCET. They are immutable once accepted; a later ADR can supersede them.

## Index

| ADR | Decision | Status |
| --- | --- | --- |
| [0001](0001-monorepo-single-version.md) | Monorepo and one product version | Accepted |
| [0002](0002-flutter-material-3-ui.md) | Flutter + Material 3 for the desktop UI | Accepted |
| [0003](0003-rust-core-in-process-ffi.md) | Rust core embedded via `flutter_rust_bridge` | Accepted |
| [0004](0004-sqlite-local-persistence.md) | SQLite as the local persistence store | Accepted |
| [0005](0005-deterministic-simulation.md) | Deterministic, virtual-time, event-driven simulation | Accepted |
| [0006](0006-offline-simulation-boundary.md) | No external integrations; simulation only | Accepted |
| [0007](0007-gpl-3.0-or-later.md) | GPL-3.0-or-later licence | Accepted |
| [0008](0008-github-only-distribution.md) | GitHub-only distribution with Linux desktop artifacts | Accepted |

## Template

```markdown
# ADR-NNNN: Title

> **Status:** Proposed | Accepted | Superseded by ADR-XXXX
> **Date:** YYYY-MM-DD
> **Decision owners:** ...

## Context
Forces, constraints, and the problem being solved.

## Decision
What we will do.

## Consequences
Positive, negative, and follow-up implications.

## Alternatives considered
Options rejected and why.
```

## Process

1. Open a proposal issue or PR containing the ADR with status `Proposed`.
2. Discuss in the PR; keep the record concise and factual.
3. On acceptance, set `Accepted`, merge, and update this index.
4. Never rewrite an accepted ADR's decision. Add a new ADR that supersedes it and update
   the old status and references.
5. If implementation reveals a better path, document the change rather than letting the ADR
   silently drift.
