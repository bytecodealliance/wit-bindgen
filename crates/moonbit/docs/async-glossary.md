# MoonBit Async Terminology

Use lowercase `future` and `stream` for Component Model types. Use uppercase
`Future` and `Stream` for local MoonBit types. `CM` may qualify a Component
Model concept in prose, but there are no public generic `CMFuture` or `CMStream`
wrapper types.

## Component Model

**Component `future`**
: A WIT `future<T>`. Its canonical value transfers a readable endpoint, not a
  MoonBit computation.

**Component `stream`**
: A WIT `stream<T>`. Its canonical value transfers a readable endpoint, not a
  local MoonBit buffer.

**Readable end / writable end**
: The receiving and producing halves of a component future or stream. Generated
  `future.new` and `stream.new` calls create the pair; WIT values carry the
  readable end.

**Endpoint site**
: One future or stream occurrence in the canonical traversal of a concrete WIT
  function. A site binds payload conversion to intrinsic names supplied by
  `wit-parser`.

**Async task scope**
: The component task context that owns a waitable set and can suspend generated
  bridge work. It is not a global event loop.

## Local MoonBit

**Local `Future[T]`**
: A consuming one-shot local value. It may hold a ready value or a lazy source,
  but its generic state contains no component endpoint or intrinsic table.

**Local `Promise[T]`**
: The producer paired with a local Future. It can complete, fail, or close local
  waiters. It is not a component future writer.

**Local `Stream[T]` / `Sink[T]`**
: A bounded local pipe. Consumers pull owned `FixedArray[T]` chunks; producers
  push immutable `ArrayView[T]` values. Capacity zero is strict rendezvous.

**Producer close**
: `Sink::close()` marks graceful EOF while preserving buffered values for
  readers.

**Consumer drop**
: `Stream::drop()` records loss of interest, wakes writers, and cleans unread
  values through the configured payload cleanup operation.

**Background group**
: The generated `background_group : @async-core.TaskGroup[Unit]` export
  parameter. Work spawned into it may continue after the component result is
  published, but cannot alter that result.

**Unstarted producer cleanup**
: An optional callback owned by a lazy Future or Stream until its producer
  starts. It releases explicitly captured resources when the unstarted value is
  rejected or dropped.

## Boundary Conversion

**Recursive boundary plan**
: Generator metadata connecting every endpoint site to endpoint sites nested in
  its payload. It drives position-specific lift, lower, commit, reject, and
  cleanup code.

**Generated readable source**
: A private lazy source that owns an incoming raw readable handle and calls that
  site's read, cancel-read, and drop-readable intrinsics.

**Generated writable state**
: Private producer state that owns an outgoing raw writable handle. It starts
  bridge work only after the paired readable end is committed to the peer.

**Staged nested endpoint**
: An endpoint revealed only at its containing transfer layer. For
  `future<future<stream<T>>>`, reading each layer reveals the next handle; the
  graph is not flattened at the function boundary.

**Prepared lower payload**
: A canonical value or buffer together with unstarted producer state and
  recursive commit/reject actions. It stays owned until the transfer outcome is
  known.

**Commit / reject**
: Commit transfers the accepted payload and starts its producer work. Reject
  cleans values and untransferred streams. A rejected future still drives its
  paired writer until a write observes reader drop.

**Future writer settlement obligation**
: After `future.new`, the writable end may be dropped only after a successful
  write or a write that reports the readable end was dropped. Cancellation does
  not settle it.

**Staged write window**
: The bounded portion of a local stream chunk lowered for one component write.
  Nested future payloads use a one-element window in the MVP.

**Accepted stream prefix**
: The count reported by a component stream operation. Accepted values belong to
  the peer; the rejected staged suffix is cleaned exactly once; unstaged values
  remain with the local stream.

**Operation buffer**
: Canonical ABI memory borrowed by an in-flight endpoint operation. It remains
  live until completion or a terminal cancellation event returns ownership.

## Cancellation And Limits

**Endpoint copy cancellation**
: Cancellation of one component read or write so its operation buffer can be
  recovered. It is separate from cancelling the MoonBit coroutine.

**Hard cancellation**
: A request to stop local task work, such as `Task::cancel` or component task
  cancellation.

**Peer loss of interest**
: The opposite endpoint was dropped. Operations report `dropped`; this does not
  imply hard cancellation of the producing coroutine.

**Sync lower producer gap**
: Lowering a newly produced Future or Stream without an active component async
  scope. The stackless callback ABI cannot resume such work after a synchronous
  call returns, so the MVP does not support it.

**BYOB / `read_into`**
: A deferred read API where callers provide destination storage to reduce
  copying. It is not part of the MVP local Stream interface.

**Endpoint operation table**
: The rejected vtable design that selected position-specific intrinsics and
  payload callbacks at runtime. The implemented design emits direct site helpers
  instead.
