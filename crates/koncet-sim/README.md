# `koncet-sim`

**Status:** Implemented for `v0.1.0`.

Deterministic simulation engine: virtual clock, command loop, seeded RNG streams, build/image
pipeline, container engine, process supervisor, reverse proxy, firewall, logs, and metrics.
Depends only on `koncet-domain`; never performs I/O or reads wall time.

See [Simulation engine](../../docs/architecture/simulation-engine.md).
