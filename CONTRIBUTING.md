# Contributing to KONCET

KONCET is designed as a long-lived, documentation-first project. Contributions are
welcome once the implementation phase begins. During the current blueprint phase,
the most useful contributions are review comments on the documents in `docs/`.

## Before you start

1. Read the [documentation index](docs/README.md).
2. Check the [PRD](docs/product/prd.md) and [non-goals](docs/product/prd.md#5-goals-and-non-goals).
   KONCET is intentionally a simulation, not an integration with real infrastructure.
3. For a substantial change, open a proposal issue first. Architecture decisions
   use the [ADR process](docs/architecture/adr/README.md).

## Development workflow

The expected workflow is trunk-based development with short-lived branches and
pull requests. The full toolchain, quality gates, and commit conventions are in
[docs/engineering/development-workflow.md](docs/engineering/development-workflow.md).

## Documentation changes

- Keep documents in English so the project remains accessible to contributors.
- Use relative links, Mermaid for diagrams, and the metadata block at the top of
  each document.
- Update the relevant document when behaviour, UI, commands, or data formats change.
- Do not document features that are not in scope without linking to the roadmap.

## Licensing

KONCET is licensed under **GPL-3.0-or-later**. By submitting a contribution you
agree that it is licensed under the same terms. There is no separate CLA.
Only add dependencies with licences compatible with GPL-3.0-or-later and permitted
by [the dependency policy](docs/engineering/development-workflow.md#9-dependency-policy).

## Reporting security issues

Do not open a public issue for a vulnerability. Follow [SECURITY.md](SECURITY.md).
