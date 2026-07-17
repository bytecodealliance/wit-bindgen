# MoonBit Async Glossary

This glossary records the vocabulary used by the MoonBit async future and stream
design notes. The naming rule is:

- lowercase code-form names, `future` and `stream`, mean component-model WIT
  types and operations;
- uppercase code-form names, `Future` and `Stream`, mean MoonBit-owned local
  async types;
- `CM` describes component-model concepts in prose. The target runtime does not
  expose reusable generic `CMFuture*` or `CMStream*` wrapper types; raw endpoint
  handles and their direct operations stay inside generated site helpers.

## Component Model Language

**Component `future`**:
The component-model `future<T>` WIT type. In a WIT signature it transfers
ownership of a readable end, not a MoonBit-local computation.
_Avoid_: `Future`, local Future

**Component `stream`**:
The component-model `stream<T>` WIT type. In a WIT signature it transfers
ownership of a readable end, not a MoonBit-owned `Stream` buffer.
_Avoid_: `Stream`, local Stream

**Readable end**:
The endpoint of a component `future` or component `stream` that can be read and
transferred through WIT.

**Writable end**:
The endpoint created with `future.new` or `stream.new` that stays in the
producing component and writes values to the paired readable end.

**Async task scope**:
The component-model task context that lets stackless MoonBit async export code
join waitables, receive callback events, and resume suspended bridge
continuations.
_Avoid_: global event loop

**Background group**:
The `background_group : @async-core.TaskGroup[Unit]` parameter intentionally
exposed by an async MoonBit export. The generated adapter publishes component
task return when the user function produces its result, then keeps the
underlying MoonBit task group alive for this hook-style work. The work remains
structurally owned inside MoonBit and bounded by the component task or instance
lifetime, but its later outcome cannot change the already-published export
result.
_Avoid_: leaked task group, detached global task

**Async core package**:
The generated `@async-core` MoonBit package. It exposes the local `Future[T]`,
`Promise[T]`, `Stream[T]`, `Sink[T]`, `Task[T]`, `TaskGroup[T]`, `Semaphore`,
`Mutex`, and `CondVar` API while keeping component-model endpoint handles and
scheduler machinery private.
Interface-specific `ffi.mbt` files remain generated boundary implementations
and are not this package.

**Sync lower producer gap**:
The unsupported case where boundary code without an active component async task
scope must lower a local Future or Stream, including recursive occurrences in a
sync import parameter or sync export result. The producer must continue after
the call returns, but the stackless callback ABI cannot resume it. This requires
cooperative component-thread support. Incoming lift is not this case because it
can remain lazy until a later async read.
_Avoid_: hidden async export

## MoonBit Binding Language

**CM future endpoint**:
A conceptual component `future` readable or writable handle owned by generated
boundary code. Its operations are statically bound to a concrete function site,
not stored in a generic MoonBit operation table.
_Avoid_: `Future`, local Future

**CM stream endpoint**:
A conceptual component `stream` readable or writable handle owned by generated
boundary code. Its operations are statically bound to a concrete function site,
not stored in a generic MoonBit operation table.
_Avoid_: `Stream`, local Stream

**Endpoint site**:
One component `future` or component `stream` node in a concrete WIT function's
canonical type traversal. A site binds its payload plan and intrinsic names to
that function context. It is generator metadata, not a runtime value.
_Avoid_: payload type alone

**Recursive boundary plan**:
The generator-owned tree that connects an endpoint site to the endpoint sites
inside its payload. It drives direct lift/lower code and recursive cleanup for
records, variants, collections, nested futures, and nested streams.
_Avoid_: endpoint vtable, flat type lookup

**Generated readable source**:
A private source closure produced when lifting an incoming component endpoint.
It exclusively owns one raw readable handle and directly calls that site's
read, cancel-read, and drop-readable intrinsics. A future source plugs into local
`Future[T]`; a stream source implements demand-driven pulls for local
`Stream[T]`.
_Avoid_: `CMFutureReader[T]`, `CMStreamReader[T]`

**Generated writable state**:
A private prepared producer state created when lowering a local Future or Stream.
It exclusively owns one raw writable handle and binds directly to that site's
write/drop intrinsics. Lowering does not start user computation. Commit starts a
normal producer task; rejection starts future writer settlement when required or
discards a prepared stream without transferring its readable end. A future state
carries a settlement obligation from the moment `future.new` succeeds.
_Avoid_: `CMFutureWriter[T]`, `CMStreamWriter[T]`

**Staged nested endpoint payload**:
The rule that only the current endpoint layer's readable handle appears in its
canonical payload. For `future<future<stream<T>>>`, the function boundary carries
the outer future handle, reading it carries the inner future handle, and reading
that carries the stream handle.
_Avoid_: flattened endpoint graph

**Transfer commit/reject**:
Generated ownership handling after lowering a payload containing resources or
nested endpoints. A successful transfer commits the transferred values to the
peer. A rejected ordinary resource is cleaned. A prepared stream drops its
untransferred component pair and rejects guest-owned local state without
pretending the peer accepted values. A rejected future drops its readable end,
but its paired writer and local source remain owned until a later write reaches
settlement. A partial stream transfer commits the accepted prefix. Generated
code retries the remainder of the current staging window while the peer remains
open; peer drop or write cancellation rejects that staged remainder exactly
once. Values beyond the staging window remain caller-owned.
_Avoid_: unconditional cleanup

**Future writer settlement obligation**:
The non-rollbackable obligation created by `future.new`. The writable end can be
dropped only after a successful write or after a write reports that the readable
end was dropped. Dropping a still-guest-owned readable end does not settle its
paired writer; the generated producer must still obtain a real payload and
attempt the write. A `cancelled` write has returned its ABI buffer but has not
settled the writer.
_Avoid_: future rollback

**Prepared lower payload**:
The private combination of a canonical ABI value or buffer, unstarted generated
producer state, and recursive `commit`/`reject` paths. Commit starts producer work
only after ownership transfers. The same lowered buffer is retained across
blocked or partial writes. For a staging window, the accepted prefix is
committed and the staged suffix is retried from the same buffer or rejected.
Both portions are consumed from the public `Sink` input; only the unstaged tail
remains caller-owned. It is not a user-facing MoonBit type.
_Avoid_: lowered value alone

**Local Future**:
A MoonBit-only one-shot handle, expected to be spelled `Future[T]`, that wraps an
owned source computation plus cancellation/drop state. Its generic state has no
CM endpoint variant and does not require `T` to be representable in WIT. A
generated readable source may capture a raw handle inside its private closure,
but the local runtime cannot inspect or forward it.
_Avoid_: component `future`, future endpoint

**One-shot async thunk**:
A MoonBit `async () -> T` value wrapped by local `Future[T]` or generated bridge
code to read or produce exactly one value. It is the computation inside the
handle, not the public component `future` representation by itself.
_Avoid_: future handle

**Future state cell**:
The local mutable runtime state owned by a `Future[T]` handle. Depending on the
constructor, it records a ready value, an owned source, or a shared local
Future/Promise settlement cell. It does not contain CM-specific states or own
writable ends.
_Avoid_: endpoint table

**Owned Future source**:
The generic pending source stored by local `Future[T]`. It has a one-shot async
`run` operation and may have an explicit unstarted cleanup action. Ordinary
MoonBit futures use a local thunk; generated incoming futures use a readable
source whose closure owns the endpoint.
_Avoid_: pending CM state

**Unstarted producer cleanup**:
The optional `on_unstarted_drop : () -> Unit` action attached to a lazy local
`Future::from` or `Stream::produce`. It owns resources captured by the producer
only until the producer starts. Drop or prepared-transfer rejection invokes it
without executing the producer; get, read, or commit clears it before producer
execution. It is not a cancellation handler for an already-running task.
_Avoid_: producer finalizer, implicit cancellation

**Bridge-owned future write**:
The separate generated task state that drives a local `Future[T]` computation
into an outgoing component `future<T>`. This state owns the raw writable handle;
the local `Future[T]` does not.
_Avoid_: local Promise, component future writer

**Future writer settlement obligation**:
The rule that once an outgoing component future pair has exposed its readable
end, the generated task owning the writable handle must eventually write one
real value or attempt the write and observe that the reader was dropped before
the writable end can be dropped. It cannot be satisfied by silently dropping
the writer.
_Avoid_: writer close

**In-flight future write**:
A generated writable operation that has already called component `future.write`
and is waiting for the `FUTURE_WRITE` waitable event. If the peer
drops the readable end while this operation is in flight, the event reports
`dropped` and returns ownership of the lowered payload buffer to the writer.
_Avoid_: bridge task in flight

**Pre-write future writer**:
A generated task that owns the writable end but has not started component
`future.write` because the payload value is not available yet. The
bridge task may be running or suspended on local work in this phase, but there
is no CM write operation waiting in a waitable set. Reader drop is observed when
the later `future.write` attempt returns or reports `dropped`.
_Avoid_: idle writer, in-flight future write

**Bridge-shielded future task**:
An outgoing future bridge task that is protected from ordinary task
cancellation after its readable end has crossed the component boundary. Peer
loss-of-interest does not cancel it; the bridge keeps running until it satisfies
the future writer settlement obligation, unless the whole component instance is
trapping or being torn down.
_Avoid_: cancellable bridge

**Generated future site helper**:
A generated function that directly calls `future.new` and the other future
intrinsics for one concrete WIT function site. It returns or consumes raw handles
only within generated boundary code.
_Avoid_: local `Future::new()`, generic component endpoint factory

**Local Promise**:
A MoonBit-only producer side paired with a local `Future[T]` by
`Future::new()` or `Future::new_with_cleanup()`. `complete` transfers a value
only if the reader still exists; `fail` and `close` settle local waiters without
a value. It never creates or owns a component `future` writable end. If its
paired Future later crosses an FFI boundary, generated code creates and owns the
separate component endpoint pair.
_Avoid_: future writer

**Local Semaphore**:
A MoonBit-only counting semaphore used for coroutine coordination. Waiters are
FIFO. If a release assigns a permit before cancellation resumes the waiter, the
permit wins that race and acquire succeeds so the permit is not lost. It is not
a component-model waitable or backpressure counter.
_Avoid_: waitable, `backpressure.inc`

**Local Mutex**:
A MoonBit-only FIFO mutex implemented by a one-permit Local Semaphore. Its
acquire operation has the same cancellation race semantics as the semaphore.
Generated stream writers use it to serialize writes and close without polling
the scheduler.
_Avoid_: component stream lock

**Local Condition Variable**:
A MoonBit-only `CondVar` used for direct task notification. If a signal has
already been assigned when cancellation resumes a waiter, the signal wins so
it cannot be lost. Generated future and stream sources use it to wait for
in-flight component read cancellation to return the operation buffer.
_Avoid_: waitable-set event, predicate polling

**Local Stream**:
A MoonBit-only stream type, expected to be spelled `Stream[T]`, that coordinates
MoonBit coroutines or wraps a generic demand-driven source without a CM-specific
state variant. A generated readable source may capture a raw component handle,
but the local runtime cannot inspect or forward it, and `T` need not be
representable in WIT.
_Avoid_: component `stream`, stream endpoint

**Local Sink**:
The producer side of a local Stream. It is not a component `stream` writable end.
_Avoid_: stream writer

**Producer close**:
The graceful local-stream terminal action performed by `Sink::close()`. It means
no more values will be written, while already-buffered values remain readable.
_Avoid_: producer cancel

**Consumer stream drop**:
The local-stream terminal action performed when the `Stream[T]` side is dropped
or explicitly consumed by async `Stream::drop()`. It means the consumer has lost
interest, so buffered values still owned by the local stream are cleaned and
producer-side waiters are woken. Bridge internals may cancel in-flight endpoint
copy operations to recover buffers, but the user-facing operation is not hard
cancellation of a producer.
_Avoid_: `Stream::cancel`, stream close

**Producer failure**:
A possible future local-stream operation where the producer ends the stream with
a failure state instead of graceful EOF. It is not `unreachable`, a wasm trap, or
process abort. It is not part of the MVP public `Sink[T]` interface because
component `stream` does not carry a distinct generic producer-failure signal.
_Avoid_: producer abort, sink close, trap

**Stream pipe**:
The shared runtime state behind a local `Stream[T]` and `Sink[T]` pair. It owns
bounded local chunk storage, FIFO waiting readers and writers, and separate
writer-close and reader-drop state. Capacity zero means strict rendezvous;
positive capacity is the maximum number of accepted unread elements. It does
not own CM stream endpoints.
_Avoid_: stream endpoint

**Owned stream chunk**:
An owned, exact-length batch of stream items returned by local `Stream[T]`
reads. The expected representation is `FixedArray[T]`; local stream
implementation should pass chunks by ownership instead of assembling them in a
growable `Array[T]`.
_Avoid_: ABI buffer

**Borrowed stream chunk**:
A temporary view of stream items, such as `ArrayView[T]`, accepted by the MVP
local `Sink[T]` write API. Before a write operation suspends, the stream runtime
must materialize the relevant view window into owned stream storage or an owned
canonical ABI buffer.
_Avoid_: stream storage

**Staged write window**:
The bounded part of a borrowed stream chunk that one awaited write operation
materializes into owned storage. MVP `Sink[T]` writes stage at most one window
per awaited operation; `write_all` loops over windows.
_Avoid_: whole input buffer

**CM operation buffer**:
The canonical ABI buffer supplied to an in-flight component `future` or
component `stream` read/write operation. The buffer is not reusable by MoonBit
until the operation completes or cancellation/drop is observed through the
terminal waitable event.
_Avoid_: local stream buffer

**Accepted stream prefix**:
The prefix length reported by a completed or terminal stream copy event. Values
inside this prefix have semantically transferred across the stream operation.
For runtime-owned buffers, the source side must not clean them again. For
caller-owned MoonBit values, this is a best-effort API contract rather than a
type-system-enforced lifetime. Values outside the prefix remain owned by the
side that staged the buffer.
_Avoid_: whole chunk

**Stream cleanup invariant**:
After every stream read, write, close, cancel, peer drop, or partial transfer,
each value is caller-owned, stream-owned, bridge-operation-owned, transferred to
the component peer, or cleaned exactly once.
_Avoid_: best-effort cleanup

**Local producer close**:
`Sink::close()` stops local writes and marks graceful EOF. Buffered chunks remain
stream-owned and can still be drained before `Stream::read` returns `None`.
_Avoid_: reader drop, cancellation

**Local reader drop**:
`Stream::drop()` records local consumer loss-of-interest. It wakes blocked
writers, discards unread chunks, and invokes the cleanup operation supplied by
`Stream::new_with_cleanup` or `Stream::produce(cleanup=...)` for each unread
owned value. A suspended
`Sink::write` owns one bounded staged `FixedArray`; `write_all` uses the same
cleanup for any remaining suffix that never entered the stream buffer.
_Avoid_: producer close, hard cancellation

**Generated stream adapter**:
Boundary-generated state connecting local and component stream semantics. An
incoming adapter is a lazy readable source and starts no pump task. An outgoing
adapter is a producer task that owns the raw writable handle and pulls from a
local `Stream[T]`.
_Avoid_: local stream state

**Incoming stream demand policy**:
The rule that a generated incoming stream source issues one component read only
for a corresponding local read demand. It does not prefetch into a local pipe.
_Avoid_: unbounded prefetch

**Source cleanup hook**:
An owned cleanup action stored with a generic local source. Explicit local
cleanup, such as async `Future::drop` or `Stream::drop`, runs the generated hook
so an idle endpoint is dropped directly or an in-flight copy is cancelled and
observed before buffers and handles are released.
_Avoid_: destructor

**BYOB stream read**:
A "bring your own buffer" read shape where the caller supplies storage to fill.
This is useful for component ABI buffers and specialized fast paths, but should
not be part of the MVP local `Stream[T]` interface for arbitrary `T`.

**Local-component bridge**:
Generated FFI-boundary code that adapts between a one-shot async thunk or local
Stream and component future/stream endpoints. It exists only for a concrete WIT
site whose payload has generated recursive lift/lower operations.

**Async generator boundary**:
The Rust generator layer that decides which MoonBit async runtime package,
endpoint bridge helpers, and public async type names to emit. This boundary
lets the old async runtime be peeled out and the replacement runtime added back
without scattering runtime-specific decisions across ordinary lift/lower code.
_Avoid_: inline async codegen

**Mergeable prototype**:
A production-intended implementation slice built after the async generator
boundary exists. It is allowed to be experimental in scope, but it must use the
real generator boundary, carry tests, and be suitable to harden in place if the
shape proves correct. It is not throwaway code.
_Avoid_: throwaway prototype

**ABI-compatible payload**:
A MoonBit type that is the generated representation of a WIT type and has the
payload lift/lower operations needed at a specific FFI boundary.
_Avoid_: arbitrary `T`

**Bridge task**:
Runtime-owned async work that drives a local-component bridge after a readable
end has been returned or passed to another component.

**Bridge continuation**:
The suspended MoonBit coroutine captured while a local-component bridge is
waiting on a component-model waitable operation. Reader drop, writer drop, and
task cancellation must be routed through this continuation rather than modeled
only as ordinary value destruction.
_Avoid_: destructor

**Endpoint copy cancellation**:
Cancellation of one in-flight component `future` or component `stream` read or
write operation. This returns ownership of the operation's buffer when the
cancelled event is observed. It is not the same thing as cancelling the MoonBit
coroutine that requested the operation.
_Avoid_: task cancellation

**Hard cancellation**:
A MoonBit-side request to stop local async work with strong semantics. Examples
include task/subtask cancellation and component teardown or failure. Hard
cancellation may cancel a producer coroutine or cancel and observe an in-flight
endpoint copy operation. It is not the same as the peer dropping a component
endpoint.
_Avoid_: peer drop, consumer loss-of-interest

**Peer loss-of-interest**:
The opposite component endpoint was dropped. When an endpoint operation observes
this, the component result is `dropped`, not `cancelled`. If there is no endpoint
operation that can observe it yet, such as a pre-write outgoing future writer,
the bridge must not infer hard cancellation from it.
_Avoid_: task cancellation

**Bridge cancellation**:
A hard cancellation of the MoonBit bridge continuation that is driving
conversion between local async work and a component endpoint. For streams and
in-flight future operations, this may cancel endpoint copy work. For a pre-write
outgoing component future writer, peer readable-end drop is not bridge
cancellation; it is observed only if a later `future.write` reports `dropped`.
_Avoid_: endpoint copy cancellation

**Future drop**:
The async user-facing operation that consumes a local `Future[T]` handle and
runs its explicit cleanup protocol. If the handle is backed by an unread component
readable end and no read is in flight, this cleanup may call
`future.drop-readable`; otherwise it may need to cancel and observe cleanup
first. If cancellation races with completion, `Future::drop` cleans the produced
value using the future's payload cleanup operation. It is not a direct alias for
component `future.drop-*`, and it is not an automatic destructor.
_Avoid_: read cancellation

**Payload cleanup operation**:
A generated or stored function that cleans a produced payload value when a
future or stream operation owns that value but will not return it to user code.
For WIT payloads, this operation is generated from lift/lower cleanup logic. A
generic local `Future[T]` or `Stream[T]` cannot invent this operation for
arbitrary `T`; local constructors therefore accept it explicitly through
`Future::ready_with_cleanup`, `Stream::new_with_cleanup`, and
`Stream::produce(cleanup=...)` when needed.
_Avoid_: generic destructor

## Adapter Generation Language

**Adapter-generated intrinsic**:
A canonical ABI import generated by the core-wasm-to-component adapter path. For
component `future` and component `stream`, these intrinsics are identified by
the containing function and a discovered component `future` or component
`stream` position.

**Function-position index**:
The enumeration index assigned to a component `future` or component `stream`
discovered in one concrete WIT function. It is not derivable from the MoonBit
payload type alone.
_Avoid_: parameter index, type index

**Endpoint operation table**:
The rejected design where adapter intrinsics and payload callbacks are stored in
a runtime record selected for a generic endpoint wrapper. The target design emits
direct site-specific calls instead.
_Avoid_: vtable

**Endpoint factory**:
A generated site helper that directly creates a component future or stream pair
because it is tied to a concrete WIT function position and therefore knows the
adapter-generated intrinsic names.
_Avoid_: local `Future::new()`, local `Stream::new()`, generic endpoint factory
