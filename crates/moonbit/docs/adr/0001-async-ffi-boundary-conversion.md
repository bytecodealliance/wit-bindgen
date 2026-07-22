# Keep local async values separate from component endpoints

Status: accepted; implemented

## Context

MoonBit `Future[T]` and `Stream[T]` must support arbitrary MoonBit values. A
Component Model `future<T>` or `stream<T>` is instead a transferable readable
endpoint whose operations and payload representation belong to a concrete WIT
function position.

The previous implementation exposed generic component endpoint wrappers backed
by operation vtables. That tied local construction and recursive conversion to
position-specific ABI machinery.

## Decision

`Future[T]`, `Promise[T]`, `Stream[T]`, and `Sink[T]` are local coordination
types and never contain component handles or operation tables.

For each endpoint occurrence, generated FFI code obtains intrinsic names from
`wit-parser`, owns the raw endpoint, and recursively lifts or lowers the payload.
Nested endpoints are converted one layer at a time when their containing value
crosses or is read at the boundary.

## Consequences

- `Future::new()` creates a local Future/Promise pair; `Stream::new()` creates a
  local Stream/Sink pair.
- Incoming endpoints become lazy generated sources. Outgoing local values are
  bridged through newly created component endpoint pairs.
- Lowering uses explicit prepare, commit, and reject paths so resources and
  partial stream prefixes transfer or clean exactly once.
- Once a component future readable end is exposed, its writer must eventually
  write a real value or observe that the reader was dropped. The binding cannot
  fabricate a default value or close it without a value.
- Position-specific endpoint traversal remains generator data, not a MoonBit
  runtime vtable.
- Producing a component future or stream requires an active component async task
  scope. Scope-free synchronous lowering remains unsupported.

Detailed API and lifecycle invariants live in the
[async design contract](../async-design.md).
