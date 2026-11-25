# `koncet-ffi`

**Status:** Implemented for `v0.1.0`.

Thin `flutter_rust_bridge` facade: DTO mapping, async commands, typed streams, batching, panic
containment, and error codes. It exposes use cases/read models, not database rows or internal
entities. Produces the `cdylib` embedded in the Flutter application.

See [Desktop bridge](../../docs/architecture/desktop-bridge.md).
