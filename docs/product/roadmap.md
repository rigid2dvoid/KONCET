# Product Roadmap

> **Status:** Baseline · **Directional, not a commitment**  
> **Last updated:** 2026-11-24  
> **Related:** [PRD](prd.md) · [Phase 1 plan](../project/phase-1-plan.md)

KONCET follows capability milestones rather than dates. A milestone closes only when
its exit criteria and documentation are complete. The single product version advances
with the monorepo; there are no separately versioned packages.

| Phase | Version | Theme | Exit criteria |
| --- | --- | --- | --- |
| 0 | — | Blueprint | Product, architecture, UX, and engineering docs baselined; contradictions resolved. |
| 1 | `0.1.0` | Single-service deployment simulator | Full loop works offline on Linux; release artifacts and notices published. |
| 1.x | `0.1.x` | Hardening | Defects, compatibility, accessibility, performance, and packaging fixes only. |
| 2 | `0.2.0` | Richer simulation and reproducibility | Multi-service topologies, replay, richer network faults, import/export. |
| 3 | `0.3.0` | Learning and portability | Scenario library, run comparison, localisation, additional Linux packaging. |

## Phase 1 — `0.1.0` Single-service simulator

**Included**

- Project and virtual source management with templates and snapshots.
- Scenario editor and four presets.
- Deterministic virtual clock, seeded simulation, command trace.
- Staged build, immutable image, container lifecycle, process supervisor.
- Deployment status derived from app health, proxy reachability, and firewall policy.
- Structured logs, timeline, and basic CPU/memory/network monitoring.
- Run history with read-only detail.
- SQLite persistence, backup, retention, single-instance and crash recovery.
- Material 3 desktop UI, light/dark/system, accessibility baseline.
- Linux AppImage and `.tar.gz`, GPL notices, checksums, GitHub Release.

**Excluded**

Real infrastructure, remote services, accounts, collaboration, mobile/web, Windows,
macOS, arbitrary code execution, automatic updates, telemetry, and background network
access.

## Phase 2 — `0.2.0` Richer simulation

Candidate scope, to be refined with an ADR and PRD amendment:

- Multiple services per deployment and multiple proxy routes.
- Container-to-container network model, service discovery aliases, and richer firewall
  evaluation.
- Run replay from the persisted command trace with step/seek controls.
- Run comparison by scenario, seed, and final state.
- Portable project/scenario bundles with versioned schema.
- Larger scenario library and guided failure exercises.
- Optional synthetic traffic profiles to exercise proxy and monitoring behaviour.

Phase 2 still must not integrate with real containers, registries, or CI/CD.

## Phase 3 — `0.3.0` Learning and portability

Candidates:

- Bahasa Indonesia localisation (`id_ID`) and translation infrastructure hardening.
- Curated in-app scenario library with explanations and expected outcomes.
- Additional packaging: `.deb` and Flatpak or Snap, if a clear maintenance model exists.
- macOS and Windows feasibility spikes, with platform-specific packaging and QA.
- Exportable run reports for portfolios or classroom use.

## Deliberately not planned

These ideas conflict with the product's identity or realistic maintenance:

- Real Docker/Kubernetes/cloud adapters.
- A plugin marketplace or arbitrary extension execution.
- Hosted accounts, sync, collaboration, or telemetry.
- Auto-update that silently performs network access.
- Emulating a specific vendor's exact implementation.
- Gamification that obscures the engineering model.

## Change control

Adding a roadmap phase or promoting a candidate requires:

1. an updated problem statement and acceptance criteria in the PRD;
2. an ADR for material architecture choices;
3. an impact assessment on determinism, storage, UI, and licensing;
4. explicit confirmation that the simulation boundary remains intact.
