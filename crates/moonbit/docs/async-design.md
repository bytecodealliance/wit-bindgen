# MoonBit Component Async Design

Status: MVP implemented on the upstream generator architecture.

This document is the design contract for MoonBit component-model `async`,
`future<T>`, and `stream<T>` bindings. It intentionally omits implementation
history. Terms are defined in the [async glossary](async-glossary.md). The two
main decisions and their detailed consequences are recorded separately:

- [ADR 0001: keep local async values detached from component endpoints](adr/0001-async-ffi-boundary-conversion.md)
- [ADR 0002: add a local Future/Promise pair](adr/0002-local-future-promise.md)

## Goals

- Give MoonBit code small, local async types that work for arbitrary MoonBit
  payloads.
- Convert those values to component endpoints only in generated code for a
  concrete WIT function position.
- Preserve canonical ABI ownership through completion, partial progress,
  cancellation, peer drop, and recursive endpoint payloads.
- Keep ordinary synchronous binding generation independent of async support.
- Align coroutine and structured-concurrency behavior with
  `moonbitlang/async` where the component protocol does not require a
  difference.

The MVP does not include component `error-context`, public BYOB/`read_into`, an
advanced raw-endpoint forwarding interface, or scope-free production of
component futures and streams from synchronous WIT calls.

## Constraints

1. MoonBit emits core wasm. `wasm-tools` adapts it into a component.
2. Future and stream intrinsics are identified by the containing WIT function
   and canonical endpoint index. A payload type `T` cannot select an intrinsic.
   Every intrinsic name must therefore come from `wit-parser`.
3. A component `future<T>` or `stream<T>` value is the readable end. Only
   generated `future.new` or `stream.new` calls create its paired writable end.
4. Canonical read and write operations borrow ABI memory until an immediate or
   terminal waitable result. Stream operations may transfer only a prefix.
5. MoonBit cannot enforce linear endpoint ownership or borrowed-view lifetimes.
   The interface must keep invalid states private and make resource cleanup
   explicit where static enforcement is unavailable.
6. A component future has no generic successful close-without-value operation.
   Once a writer is exposed, it must write a real value or attempt that write
   and observe reader drop before the writable end can be dropped.
7. Stackless callback exports can resume suspended work only inside an active
   component async task scope.

## Architecture

The design has three layers.

### Local runtime

The generated `@async-core` package owns source-language concepts:

- `Future[T]`, `Promise[T]`, `Stream[T]`, and `Sink[T]`;
- `Task[T]`, `TaskGroup[T]`, cancellation shielding, and coroutine scheduling;
- `Semaphore`, `Mutex`, and `CondVar`;
- payload-independent waitable event decoding.

It does not contain generic component endpoint types, endpoint vtables, or a
generic `future.new`/`stream.new` operation.

### Recursive generator plan

For each WIT function, the Rust generator builds a recursive endpoint plan from
the canonical type shape and `wit-parser`'s ordered future/stream occurrences.
Each plan node records:

- whether it is a future or stream;
- its position-specific intrinsic names from `wit-parser`;
- child endpoint occurrences in its payload;
- payload lift, lower, commit, reject, and cleanup behavior.

This is generator data, not a runtime table.

### Generated site helpers

Generated `ffi.mbt` code owns raw readable and writable handles, canonical ABI
buffers, and copy-operation state. A helper directly calls the intrinsics for
one plan node and converts between that endpoint and a local value.

The deletion boundary remains important: endpoint-free synchronous worlds do
not emit the async runtime or wrappers, and their generated output remains
unchanged as async support evolves.

## Public Interface

The main local interface is:

```mbt
async fn Future::get(self : Future[T]) -> T
async fn Future::drop(self : Future[T]) -> Unit
fn Future::ready(value : T) -> Future[T]
fn Future::ready_with_cleanup(value : T, cleanup : (T) -> Unit) -> Future[T]
fn Future::from(
  producer : async () -> T,
  on_unstarted_drop? : () -> Unit,
) -> Future[T]

fn Future::new() -> (Future[T], Promise[T])
fn Future::new_with_cleanup(
  cleanup : (T) -> Unit,
) -> (Future[T], Promise[T])
fn Promise::complete(self : Promise[T], value : T) -> Bool
fn Promise::fail(self : Promise[T], error : Error) -> Bool
fn Promise::close(self : Promise[T]) -> Bool

fn Stream::new(capacity? : Int = 0) -> (Stream[T], Sink[T])
fn Stream::new_with_cleanup(
  cleanup : (T) -> Unit,
  capacity? : Int = 0,
) -> (Stream[T], Sink[T])
fn Stream::produce(
  producer : async (Sink[T]) -> Unit,
  cleanup? : (T) -> Unit,
  on_unstarted_drop? : () -> Unit,
) -> Stream[T]
async fn Stream::read(self : Stream[T], max : Int) -> FixedArray[T]?
async fn Stream::drop(self : Stream[T]) -> Unit

async fn Sink::write(self : Sink[T], values : ArrayView[T]) -> Int
async fn Sink::write_all(self : Sink[T], values : ArrayView[T]) -> Bool
fn Sink::is_open(self : Sink[T]) -> Bool
async fn Sink::close(self : Sink[T]) -> Unit
```

`Sink[Byte]` additionally provides `write_bytes(BytesView)` and
`write_all_bytes(BytesView)` so immutable `Bytes` do not require an `Array`
conversion.

`Future[T]` is one-shot. `Promise[T]` is local coordination, never a component
writable endpoint. Expected cross-component failure belongs in `T`, for example
`Future[Result[V, E]]`; local `Promise::fail` and `Promise::close` cannot settle
an exposed component future without a value.

Streams support both directions without exposing endpoint machinery:

- consumers pull owned `FixedArray[T]` chunks from `Stream[T]`;
- producers push immutable `ArrayView[T]` values into `Sink[T]`;
- the runtime copies a bounded staging window before suspension;
- capacity is measured in elements, with zero meaning strict rendezvous;
- `write` returns the consumed prefix length. When cleanup is configured,
  consumption includes values accepted by the peer and staged values recursively
  cleaned after peer drop;
- `is_open` distinguishes a fully accepted write from one that consumed values
  while closing, and `write_all` returns whether the endpoint remained open
  through the operation;
- `Sink::close` is graceful producer completion and preserves buffered values;
- `Stream::drop` is consumer loss-of-interest and cleans unread owned values.

There is no public `Sink::cancel`. Component streams have no distinct generic
producer-failure signal to preserve. Such failure belongs in the payload or
surrounding WIT protocol.

Async export implementations also receive:

```mbt
background_group : @async-core.TaskGroup[Unit]
```

The generated adapter publishes component task return when the implementation
returns its result, then allows this group to finish hook-style work. The work
remains structurally owned in MoonBit but cannot alter the published result.

## Boundary Conversion

The same four conversions cover import parameters/results and export
parameters/results:

| Component value | Local value | Generated owner |
| --- | --- | --- |
| Incoming `future<T>` readable | lazy `Future[T]` source | source closure owns readable and read state |
| Outgoing `Future[T]` | new component future pair | producer task owns writable and settlement |
| Incoming `stream<T>` readable | demand-driven `Stream[T]` source | source closure owns readable and read state |
| Outgoing `Stream[T]` | new component stream pair | producer task owns writable and staged writes |

Incoming conversion is lazy and creates no endpoint pair. `Future::get` or
`Stream::read` calls the concrete site's read intrinsic when the user requests
data. Dropping the local value calls the same site's cancel/drop path if needed.

Outgoing conversion first prepares a pair. The readable end may be nested in a
larger canonical payload, so producer work starts only after that payload's
ownership disposition is known:

- commit means the peer owns the readable end and starts normal production;
- reject cleans an untransferred stream pair and local stream state;
- rejecting a future drops its readable end but still drives the paired writer
  until a write observes that reader drop.

### Recursive endpoints

A WIT value such as:

```wit
future<future<stream<T>>>
```

maps directly to:

```mbt
Future[Future[Stream[T]]]
```

The local types contain no CM index. The recursive generator plan binds each
layer to its own site. Reading the outer future reveals the next readable
handle; reading that future reveals the stream handle. Conversion is staged,
not flattened at the original function boundary.

Every lowered nested payload has explicit commit and reject actions. For a
partial `stream<future<T>>` write, the accepted prefix belongs to the peer; a
staged rejected suffix is settled by generated cleanup, while an unstaged tail
is cleaned by `Sink`. All three are included in the consumed prefix returned to
the caller, which must not retry any of them. The MVP stages one such element at
a time so backpressure cannot create an unbounded number of unsettled future
writers.

## Runtime Invariants

### Ownership

- Every raw endpoint has exactly one owner: generated source state, generated
  producer state, an in-flight canonical transfer, or the peer.
- A copy-operation buffer remains live until an immediate or terminal result.
  Calling `cancel-*` alone does not return the buffer.
- A completed stream copy transfers exactly its canonical reported prefix. Any
  staged suffix is rejected and recursively cleaned exactly once; `Sink::write`
  reports both portions as consumed so aliases cannot retry rejected values.
- Generated cleanup recursively follows the active record, tuple, variant,
  list, resource, future, and stream shape.
- Rejecting a materialized local stream installs generated payload cleanup on
  its pipe before waking writers, so buffered values, pending writer values,
  and writes attempted after rejection are each cleaned exactly once.
- `Future::new_with_cleanup`, `Stream::new_with_cleanup`, and
  `Stream::produce(cleanup=...)` clean accepted local values discarded before
  consumption. Variants without element cleanup are appropriate only when
  discarded `T` needs no explicit cleanup.
- Lazy `Future::from` and `Stream::produce` may carry `on_unstarted_drop` for
  captured resources. `Stream::produce` separately accepts per-element cleanup
  for values written after production starts.

Ownership through `ArrayView[T]` is a best-effort contract because MoonBit
cannot prevent aliases. Callers must treat the consumed prefix as moved, whether
the peer accepted it or generated cleanup rejected it. `write_all` additionally
assumes disposal responsibility for the unstaged tail and cleans it before
returning `false` or propagating an error.

### Cancellation

Three events must remain distinct:

- MoonBit task cancellation cooperatively stops local work;
- endpoint copy cancellation recovers an in-flight operation buffer;
- peer drop reports loss-of-interest through a read or write result.

An incoming read cancelled by MoonBit first issues the concrete endpoint
cancel operation, waits for its terminal event, reclaims the buffer, and only
then drops the readable end. `CondVar` provides direct cleanup notification;
the runtime must not poll by repeatedly yielding.

The generated source records the component task that registered an in-flight
read. If another export drops that source, cancellation is routed to the
original task's waitable set before the reader is woken.

An outgoing stream serializes writes and close with `Mutex`, because the
canonical ABI permits only one active operation on an endpoint. MoonBit task
cancellation issues the concrete `stream.cancel-write`, waits for buffer
ownership to return, commits any transferred prefix, rejects the staged suffix,
and then drops the writable end. Unlike a future, a stream can terminate without
inventing a payload value.

An exposed outgoing future writer is a settlement obligation. Its producer is
shielded from ordinary task/subtask cancellation until it writes one real value
or a write reports reader drop. If its local future never produces `T`, no
generic cleanup can settle it. The writer may remain pending until instance
teardown; the binding does not fabricate a default value.

### Scheduling

- Each top-level component task owns a distinct waitable set and scheduler
  queue, preventing concurrent exports from consuming each other's events.
- A scheduling round runs only work ready at its start, preserving an event-loop
  poll opportunity between rounds.
- Repeated wakes are deduplicated.
- A task cancelled before first execution still enters its body so entry cleanup
  can be installed.
- `TaskGroup` failure propagation and cancellation shielding follow
  `moonbitlang/async` semantics.
- Canonical `backpressure.inc/dec` controls admission of component tasks. It is
  not tied to endpoint producer lifetime; read/write suspension already provides
  data-flow backpressure.

## Synchronous WIT Functions

WIT sync functions remain sync. The generator does not silently give them a
stackless async callback ABI.

Incoming endpoint lift is lazy, so a sync function can receive a readable end
without immediately starting a copy. Reading it later still requires an async
task scope.

Lowering a local `Future` or `Stream` requires an active component async task
scope. A sync import called from such a scope is supported: generated code
prepares endpoint arguments, performs the core call, and commits those handles
immediately afterward. Scope-free sync imports and sync export results that
create producer work are unsupported. Supporting them requires cooperative
component threads, not a callback-ABI workaround.

Raw endpoint identity forwarding is also not implicit. A future advanced
interface may provide it explicitly without weakening the ordinary local types.

## `moonbitlang/async` Alignment

The runtime is audited against `moonbitlang/async` main at commit `18533c8d`.
The continuation primitive, cancellation races, shielding, wake behavior,
fairness, `Task`, `TaskGroup`, `Semaphore`, `Mutex`, and `CondVar` semantics are
kept aligned.

Intentional differences are limited to the component environment:

- component waitable sets replace the platform event loop;
- scheduler state is partitioned by waitable set;
- generated bridge work is owned by the component task's waitable set, not a
  user `TaskGroup`, so lazy producers can continue after the export returns;
- public timers, retry, async queues, `spawn_loop`, and `pause` are omitted;
- local Future/Promise and Stream/Sink encode WIT ownership needs not provided
  by `moonbitlang/async`.

## Supported MVP

Implemented and covered by composed runtime tests:

- async imports and exports;
- local Future/Promise and Stream/Sink coordination;
- incoming and outgoing component futures and streams;
- nested endpoint payloads in both directions;
- partial stream progress, concurrent writes, cancellation races, and resource
  payload cleanup;
- concurrent component-task isolation;
- `wasi:cli@0.3.0` stream output;
- `wasi:http@0.3.0` body, trailers, and post-response background work;
- deletion guards proving endpoint-free sync generation is unchanged without
  async support.

Known limits:

- component `error-context` is out of scope;
- public BYOB/`read_into` is deferred;
- scope-free sync lowering is unsupported;
- fixed-length WIT lists use runtime-checked `FixedArray[T]`; combining one
  with a future or stream in either nesting direction is not yet supported;
- a future producer that never produces a value cannot settle its component
  writer generically;
- explicitly captured resources require `on_unstarted_drop` before lazy
  production; lazy streams of managed values also require per-element cleanup
  after production starts;
- generated glue and user implementations still share a MoonBit package, so
  `#internal` is documentation control rather than an access boundary.

## Deferred Decisions

1. Whether named WIT future/stream aliases should become distinct MoonBit names.
2. Whether to expose a site-aware endpoint-forwarding interface.
3. Whether `Sink::close` needs a separate downstream `finish`/`flush` operation.
4. Whether measured workloads justify BYOB or larger nested-future stream
   staging windows.
