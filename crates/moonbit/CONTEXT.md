# MoonBit Bindings Context

This context records shared language for the MoonBit binding generator. Async
future and stream vocabulary is split into [Async Glossary](docs/async-glossary.md).

## Language

**MoonBit binding**:
Generated MoonBit source that exposes WIT imports to MoonBit code or adapts
MoonBit exports to the component ABI.

**Component adapter path**:
The current implementation path where MoonBit emits core wasm and `wasm-tools`
converts it into a component using adapter imports and exports.
_Avoid_: direct component generation

## Async Design

- [Async design contract](docs/async-design.md)
- [Async glossary](docs/async-glossary.md)
- [ADR 0001: FFI-boundary conversion](docs/adr/0001-async-ffi-boundary-conversion.md)
- [ADR 0002: local Future/Promise pair](docs/adr/0002-local-future-promise.md)

The implementation targets the official upstream generator architecture. WIT
`future` and `stream` remain distinct from local MoonBit `Future` and `Stream`;
generated code converts them only at concrete FFI positions whose intrinsic
names are supplied by `wit-parser`. Local `Future::new()` returns a
MoonBit-only Future/Promise pair, and local `Semaphore` coordinates coroutines;
neither can select or own a component endpoint from `T`. Async support is always
available in the MoonBit generator, but endpoint-free synchronous worlds do not
emit its runtime or wrappers. Component endpoint wrappers are
generated-code-only, enforce a single in-flight operation, and retain operation
buffers until cancellation or completion is observed. Each top-level component
task has its own waitable set and scheduler state.
