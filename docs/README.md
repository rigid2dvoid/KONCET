# KONCET Documentation

> **Status:** Phase 0 — Baseline for `v0.1.0`  
> **Last updated:** 2026-11-24

This directory is the source of truth for KONCET's product intent, architecture,
interaction design, engineering process, and release plan. The application has not
been implemented yet; these documents are the contract that implementation must
satisfy.

## Reading paths

| If you are… | Read, in order |
| --- | --- |
| Evaluating the product | [PRD](product/prd.md) → [Use cases](product/use-cases.md) → [Roadmap](product/roadmap.md) |
| Implementing the core | [Architecture overview](architecture/overview.md) → [Simulation engine](architecture/simulation-engine.md) → [Domain model](architecture/domain-model.md) → [Data model](architecture/data-model.md) |
| Building the UI | [Design system](design/design-system.md) → [Screens and flows](design/screens-and-flows.md) → [Desktop bridge](architecture/desktop-bridge.md) |
| Preparing a release | [Monorepo](engineering/monorepo.md) → [Testing](engineering/testing-strategy.md) → [Release and distribution](engineering/release-and-distribution.md) |
| Contributing | [Development workflow](engineering/development-workflow.md) → [Phase 1 plan](project/phase-1-plan.md) |

## Structure

```text
docs/
├── product/        Problem, users, scope, requirements, journeys, roadmap
├── architecture/   System structure, engine, domain, data, FFI, security, ADRs
├── design/         Brand, Material 3 tokens, components, screens, UX flows
├── engineering/    Monorepo, workflow, testing, release and distribution
├── project/        Executable phase plan and delivery gates
└── reference/      Scenario format, event catalogue, glossary
```

## Document index

| Area | Documents |
| --- | --- |
| Product | [PRD](product/prd.md) · [Use cases](product/use-cases.md) · [Roadmap](product/roadmap.md) |
| Architecture | [Overview](architecture/overview.md) · [Stack](architecture/stack.md) · [Simulation engine](architecture/simulation-engine.md) · [Domain model](architecture/domain-model.md) · [Data model](architecture/data-model.md) · [Desktop bridge](architecture/desktop-bridge.md) · [Security](architecture/security.md) · [ADRs](architecture/adr/README.md) |
| Design | [Design system](design/design-system.md) · [Screens and flows](design/screens-and-flows.md) |
| Toolchain | [Toolchain versions](engineering/toolchain.md) |
| Engineering | [Monorepo](engineering/monorepo.md) · [Workflow](engineering/development-workflow.md) · [Testing](engineering/testing-strategy.md) · [Release](engineering/release-and-distribution.md) |
| Project | [Phase 1 plan](project/phase-1-plan.md) |
| Reference | [Scenario schema](reference/scenario-schema.md) · [Event catalogue](reference/event-catalog.md) · [Glossary](reference/glossary.md) |

## Conventions

- Documents are written in English and use RFC 2119 keywords (`MUST`, `SHOULD`,
  `MAY`) where requirements are normative.
- Each document starts with a status block. Statuses are:
  - **Draft** — under discussion; not safe to implement.
  - **Baseline** — approved direction for the stated release; implementation must
    conform or raise a change.
  - **Decision** — an accepted ADR; supersedes earlier conflicting text.
  - **Superseded** — retained for history; links to its replacement.
- Diagrams use Mermaid and live in the Markdown source.
- Links are relative so the documentation works offline and in GitHub.
- Terminology is defined in the [glossary](reference/glossary.md). Product copy and
  code identifiers should use those terms consistently.

## Document precedence

When documents conflict, resolve in this order:

1. Accepted ADRs in [`architecture/adr/`](architecture/adr/README.md).
2. Architecture contracts (`simulation-engine.md`, `data-model.md`, `desktop-bridge.md`).
3. The PRD and reference schemas.
4. Design and engineering guidance.
5. Root-level prose.

Conflicts must be fixed in the same change that reveals them. Silent divergence between
documentation and implementation is treated as a defect.

## Maintaining the documentation

- Update the affected document in the same pull request as a behaviour or interface change.
- Changing a normative requirement requires a PRD change and, where relevant, an ADR.
- Changing persisted data requires a migration plan and an update to the data model.
- Changing the engine's observable behaviour requires a golden-run update, an engine
  compatibility note, and usually a version bump.
- Add an ADR only for decisions that are costly to reverse or shape multiple components.
  Do not use ADRs for routine implementation details.
