# ADR-0006: Offline Simulation Boundary

> **Status:** Accepted · **Date:** 2026-11-24

## Context

The product exists to make deployment practice safe and repeatable. Integrating real Docker,
cloud providers, Git hosting, or CI systems would introduce credentials, network failures,
host privileges, and version drift. It would also blur whether actions are simulated, harming
user trust and the portfolio's product identity. The requested scope is explicitly a
“full simulation with no link to external applications”.

## Decision

KONCET is permanently **offline and simulation-only at runtime**.

- No real container runtime, build execution, proxy, process manager, firewall, registry, or
  network socket.
- No telemetry, analytics, remote logging, auto-update, remote assets, or online accounts.
- All assets are bundled; all data is local.
- The simulation boundary is enforced by dependency policy, architecture tests, and a
  network-syscall smoke test.
- Exports of user data, if added later, are user-initiated files, not integrations.
- The UI labels simulated environments and uses reserved `.koncet.local` hostnames.

## Consequences

**Positive:** strong safety story; reproducible environments; simple packaging; privacy by
default; no operational credentials; durable educational value; clear product positioning.

**Negative:** cannot validate against real-world quirks; some users may want real adapters;
fidelity must be designed rather than observed; updates and support are manual.

**Mitigations:** be explicit about fidelity in product copy; make models internally coherent
and deterministic; document limitations; keep a strict adapter-free architecture; collect
feedback without telemetry.

## Alternatives considered

- **Optional real Docker adapter:** greatly increases scope, security surface, packaging, and
  support burden; undermines the product's safe-practice identity for `v0.1` and beyond.
- **CI/CD provider integrations:** require credentials, network, remote state, and constant
  API maintenance; explicitly out of scope.
- **Hybrid “simulate then apply”:** creates real-world risk and trust ambiguity; rejected.
- **Cloud-hosted simulator:** violates local ownership, offline operation, and privacy principles.
