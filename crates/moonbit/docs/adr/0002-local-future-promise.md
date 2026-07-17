# Add a local Future/Promise pair

Status: accepted; implemented in the current branch

MoonBit needs a producer-facing one-shot primitive for local control-flow cycles
that cannot be expressed as `Future::from(async () -> T)`. The motivating case
is `wasi:http@0.3.0` request handling: `consume-body` takes a Future reporting
processing completion before it returns the request body whose later processing
determines that completion.

`Future::new()` returns `(Future[T], Promise[T])` and
`Future::new_with_cleanup(cleanup)` adds explicit value-discard cleanup. The
Promise can complete with a value, fail with a MoonBit error, or close without a
value. It is local coordination state only. It does not create, wrap, or own a
component `future` endpoint, and it works for arbitrary MoonBit `T`.

## Consequences

- `Promise::complete(value)` returns `true` only when the reader accepts
  ownership. If the Future was already dropped, it returns `false` and the
  caller retains `value`.
- `Future::drop()` after accepted completion runs the cleanup supplied to
  `new_with_cleanup`. The plain `new()` constructor is appropriate only when
  discarded `T` needs no explicit cleanup.
- `Promise::fail(error)` makes local `Future::get()` raise that error.
  `Promise::close()` makes it raise `PromiseClosed`.
- Settlement is one-shot. Repeating `complete`, `fail`, or `close` after a
  successful settlement is a programmer error.
- Dropping or otherwise abandoning a still-pending Promise does not implicitly
  close its Future because MoonBit has no generic deterministic destructor. The
  producer must explicitly complete, fail, or close it.
- Task cancellation that reaches a waiting reader before settlement drops the
  reader, so later completion returns `false`. Once completion assigns the value
  and wakes the reader, completion wins a simultaneous cancellation race and the
  reader receives the value.
- Explicit `Future::drop()` follows the same race rule while `get()` is pending:
  dropping before settlement wakes the reader with `Cancelled`, while an
  already-assigned value or error remains owned by the waiting reader.
- A local failure or close cannot settle an already-exposed component future
  without a value. If such an outcome is expected across WIT, it belongs in the
  payload type, for example `Future[Result[V, E]]`.
- Generated FFI-boundary code remains solely responsible for creating concrete
  component future pairs and satisfying their writable-end settlement rules.
