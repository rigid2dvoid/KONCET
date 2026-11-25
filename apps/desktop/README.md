# `apps/desktop` — Flutter desktop application

**Status:** Implemented for `v0.1.0`.

This directory will contain the Linux Flutter shell, Material 3 screens, Riverpod providers,
navigation, and the generated Rust bridge bindings. It depends on `packages/koncet_ui` and the
Rust core, and must not contain engine rules, SQL, or direct filesystem persistence.

See [Screens and flows](../../docs/design/screens-and-flows.md) and
[Desktop bridge](../../docs/architecture/desktop-bridge.md).
