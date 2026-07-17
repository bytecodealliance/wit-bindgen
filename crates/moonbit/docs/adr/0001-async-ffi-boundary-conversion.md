# Keep MoonBit async values detached from component futures and streams

Status: accepted; implemented in the current branch

MoonBit `Future[T]` handles and local `Stream[T]` values are over arbitrary
MoonBit types, while component `future<T>` and `stream<T>` are ABI values over
WIT-representable payloads. MoonBit async bindings will keep those concepts
separate: generated FFI-boundary code converts between local async values and
component endpoints only at concrete WIT positions where the endpoint operations
and payload lift/lower code are known.

For MVP, component `future<T>` maps to a one-shot `Future[T]` handle. The handle
represents a ready value or an owned local source computation with
value-discarding drop cleanup. For an incoming component future, generated code
supplies a source closure that captures the raw readable handle and directly
binds the concrete WIT position's read/cancel/drop intrinsics. The generic local
type has no CM-specific variant or operation table. A public local `Promise[T]`
may settle the same Future state, but it is not a component writable endpoint.

## Consequences

- `Stream::new()` creates local MoonBit values only.
- `Future::new()` creates a local `Future[T]` / `Promise[T]` pair only. It does
  not select or invoke a component `future.new` intrinsic.
- Endpoint operation tables are not part of the target design. Generated
  position-specific helpers call canonical intrinsics directly and keep raw
  handle state inside generated source closures or producer tasks.
- Component intrinsic module and field names are generated through
  `wit-parser`'s `WasmImport::{FutureIntrinsic, StreamIntrinsic}` API. The
  MoonBit generator does not reproduce position indices, export prefixes,
  unit-payload names, or async-lower prefixes by string formatting.
- The Rust generator boundary owns a recursive conversion plan for each
  function. Ordinary lift/lower code requests position-specific helpers from
  that plan instead of naming runtime endpoint types or type-shaped tables.
- The existing async generator boundary is retained. The static recursive
  rewrite happens behind it without changing ordinary lift/lower ownership or
  the endpoint-free sync path.
- User-facing bindings expose local `Future[T]` handles and local `Stream[T]`
  for ordinary async composition.
- A nested WIT shape such as `future<future<stream<T>>>` maps to
  `Future[Future[Stream[T]]]`. Only the current layer's readable handle appears
  at each canonical payload stage. Generated recursive lift/lower functions bind
  each layer to its own function-position intrinsic and apply generated
  commit/reject dispositions at every transfer boundary.
- Recursive lower retains its canonical buffer and prepared producer state until
  the transfer reports a disposition. Commit starts producer work only after
  ownership transfers. Stream progress commits the accepted prefix and retries
  the same lowered suffix; an abandoned suffix is rejected exactly once.
- A freshly-created component future is not fully rollbackable. Canonical ABI
  permits its writable end to be dropped only after a write succeeds or a write
  reports that the reader was dropped. If an outer transfer rejects a nested
  future readable, generated code drops that readable but must still drive the
  paired writer with the local future value until its write observes `dropped`.
  A cancelled write is not settlement and is retried while the component task
  remains alive.
- Stream batches whose element type recursively contains a future use a
  one-element staging window for MVP. This bounds settlement obligations created
  before downstream acceptance without changing the public stream API.
- Generated stream producers remain prepared when their component pair is
  created. Commit starts normal pumping. Parent rejection drops both
  untransferred component ends and performs state-aware local rejection: incoming
  component sources close immediately, buffered values use configured cleanup or
  generated cleanup as fallback, and an unstarted local producer is discarded
  without executing user code. `Stream::produce` accepts an optional
  `on_unstarted_drop` callback for resources captured by that branch and a
  separate per-element cleanup for values written after it starts.
- Canonical `backpressure.inc/dec` controls admission of new async component
  tasks. It is not tied to future/stream bridge lifetime and is not called
  implicitly by the MoonBit runtime. Endpoint read/write suspension provides
  data-flow backpressure independently.
- Async import argument settlement distinguishes `cancelled-before-started`
  from every state in which the callee may have started. The former recursively
  rejects owned resources and endpoints; the latter only reclaims guest-owned
  canonical list allocations.
- Local future MVP has consuming `Future::get()` and value-discarding async
  `Future::drop()`. Strong cancellation belongs to `Task` and `TaskGroup`, not
  to a future-specific result type. `Future::drop()` is explicit cleanup, not a
  direct alias for component `future.drop-*`, and futures that may discard
  completed WIT payloads carry generated payload cleanup logic.
- Outgoing component `future<T>` producer tasks must settle their raw writable
  handle by writing a real value or by attempting the write and observing reader
  drop. The MVP does not fabricate default values to satisfy unwritten futures.
  If user code produces a ready `T`, generated code may write immediately; if
  user code returns a pending `Future[T]`, the bridge exists before the CM
  `future.write` operation starts because the component boundary already needs a
  readable end to return.
- After an outgoing component `future<T>` readable end is committed, or after a
  parent transfer rejects and locally drops it, the bridge shields producer work
  from ordinary task/subtask cancellation. Component-task cancellation is
  cooperative: it may resolve the cancelled call with `task.cancel`, but it does
  not forcibly destroy shielded settlement work. Only instance teardown or a
  trap can abandon the writer without settlement. Peer reader drop is
  loss-of-interest, not task cancellation; before `future.write` has started,
  the bridge does nothing special for it. If a later write reports `dropped`,
  the bridge cleans the value and settles the writer.
- If that local future never produces `T`, the Component Model provides no
  generic close-without-value operation. This is an explicit liveness limit; the
  binding does not fabricate a default value or pretend an idle writer can be
  dropped safely. The same limit applies when the paired readable was created
  for a nested payload but the outer transfer rejected it before ownership
  crossed the boundary.
- Local stream MVP has `Sink::close()` for graceful producer close and async
  `Stream::drop()` or `Stream[T]` drop for consumer loss-of-interest. It does
  not expose public `Sink::cancel()` because component `stream` has no distinct
  generic producer-failure signal to preserve across the boundary.
- Local stream state keeps producer close separate from reader drop. Producer
  close preserves buffered `FixedArray[T]` chunks for draining; reader drop
  discards unread chunks and uses an explicit `Stream::new_with_cleanup` or
  `Stream::produce(cleanup=...)` callback for payloads that need resource
  cleanup.
- Local stream capacity is measured in elements. Zero is strict rendezvous and
  a positive value is a hard bound on accepted unread values; local streams are
  never implicitly unbounded. Waiting readers and writers use direct FIFO
  handoff with completion-versus-cancellation race handling.
- An incoming component stream uses a generated demand-driven source whose
  reads directly call its concrete site intrinsics. It does not start an eager
  pump into a local stream pipe.
- Forwarding component endpoints without reading them is not the default path and
  needs an explicit advanced API if we decide to support it.
- Runtime validation includes real `wasi:cli@0.3.0` stream output and a real
  `wasi:http@0.3.0` handler response whose body stream, trailers future, and
  transmission completion continue after the handler returns the response.
- Local stream validation covers strict rendezvous, bounded buffering, cancelled
  waiter removal, completion-versus-cancellation ownership races, local
  producer reads, and unread resource cleanup.
- Async export stubs intentionally expose
  `background_group : @async-core.TaskGroup[Unit]`. It adapts the mismatch
  between MoonBit structured completion and component task return. Generated code
  publishes component task return when the user function produces its result,
  then keeps the underlying MoonBit task group alive for hook-style post-return
  work. That work remains structurally owned and bounded by the component task
  or instance lifetime, but cannot change the already-published export result.
- Lowering local `Future[T]` and `Stream[T]` values requires an active component
  async task scope, including recursive occurrences inside structured payloads.
  A sync WIT import called in that scope prepares its endpoint arguments and
  commits those same handles immediately after the core call returns. Sync export
  results, and sync imports called without an active scope, remain unsupported.
  Incoming lift remains lazy and does not require a producer task until user code
  later reads in an async scope. Scope-free sync lowering requires cooperative
  component-thread support and must not be faked by using the stackless callback
  ABI for a non-async WIT function.
