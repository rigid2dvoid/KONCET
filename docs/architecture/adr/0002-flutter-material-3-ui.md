# ADR-0002: Flutter and Material 3 for the Desktop UI

> **Status:** Accepted · **Date:** 2026-11-24

## Context

KONCET is a data-dense Linux desktop workbench with a strong visual identity, accessible
interaction, charts, a code editor, tables, and fast feedback during simulation. The product
needs a consistent component system from the first release and a path to macOS/Windows later.
The UI is not a website and must work fully offline.

## Decision

Use **Flutter with Dart** and **Material 3** as the native desktop UI.

- Pin a Flutter stable version in the repository when scaffolding begins.
- Use a fixed Material 3 colour scheme derived from a single KONCET seed; do not use dynamic
  wallpaper colours in `v0.1`.
- Encapsulate tokens and shared components in `packages/koncet_ui`.
- Target Linux first, with desktop-specific window management, keyboard navigation, and
  minimum dimensions. Do not design a mobile layout.
- Bundle all fonts, icons, and assets; load nothing from the network.

## Consequences

**Positive:** productive custom UI; consistent theming and accessibility primitives; strong
golden/widget testing; one UI codebase for future desktop platforms; large ecosystem.

**Negative:** large runtime/bundle size; GTK/glibc compatibility matters; Flutter's text
rendering and IME behaviour need Linux QA; bridge complexity with Rust; some packages may lack
Linux support.

**Mitigations:** reference hardware budgets, AppImage baseline testing, dependency review,
native code-editor spike, and strict separation between presentation and engine logic.

## Alternatives considered

- **Qt/C++:** excellent native desktop control but slower UI iteration and higher memory-safety
  burden.
- **GTK4 + Rust:** strong Linux integration but weaker cross-platform direction and fewer
  polished data-visualisation components.
- **Tauri + web frontend:** small binaries but webview inconsistency, web security surface,
  and weaker Material 3 fidelity.
- **`egui` / immediate mode:** excellent for tools but less mature for forms, accessibility,
  and the intended polished product experience.
- **Web application:** conflicts with the offline desktop product and local-first positioning.
