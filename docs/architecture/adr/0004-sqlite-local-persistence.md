# ADR-0004: SQLite for Local Persistence

> **Status:** Accepted · **Date:** 2026-11-24

## Context

KONCET stores projects, virtual source files, immutable snapshots, scenarios, runs, ordered
events, logs, metric samples, and settings. The data is local, single-user, relational, and
queried with filters and pagination. History must survive updates and crashes. No external
database or service is allowed. The Rust core owns persistence, and tests must be fast.

## Decision

Use **SQLite** through **`rusqlite` with the bundled SQLite build**, in WAL mode.

- One database file in the XDG data directory.
- One writer thread; bounded read-only pool; no UI-thread database access.
- Embedded, append-only SQL migrations with checksums and pre-migration backups.
- `STRICT` tables, foreign keys enabled, parameterised SQL, UUIDv7 text IDs.
- Schema version 1 is defined in the data-model document; changes require migrations and
  tests.
- Logs and metrics are stored in SQLite for `v0.1` with per-run caps.

## Consequences

**Positive:** transactional integrity, expressive filtering, no server, single-file backup,
mature tooling, excellent test support with temporary/in-memory databases, bundled library
avoids host variance.

**Negative:** single-writer bottleneck; large log tables need caps and indexing; JSON columns
shift some validation to application code; migration discipline is mandatory; file corruption
or user data loss remains possible without backups.

**Mitigations:** batched writes, WAL, keyset pagination, caps, integrity checks, pre-migration
backup, documented manual restore, and future FTS5/archival only if benchmarks justify it.

## Alternatives considered

- **JSON files per project:** simple to start but weak transactions, querying, ordering, crash
  safety, and log pagination.
- **Embedded key-value store (sled, RocksDB):** poor relational queries and migration ergonomics.
- **PostgreSQL/MySQL:** external service, violates offline/self-contained principle.
- **Dart `drift` as the primary store:** duplicates persistence across languages and separates
  data ownership from the Rust engine; the UI should not own storage.
- **Multiple SQLite databases per run:** complicates cross-run history and deletion without a
  meaningful scaling benefit.
