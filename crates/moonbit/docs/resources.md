# Resource implementations and handles

The resource author implements the original WIT type name. For an exported
`counter`, this is `Counter`; the bindings generate `CounterHandle` for owning,
transferring, and dropping the resource. An importing component sees just
`Counter`, a generated proxy whose methods call the implementing component.

Implement the declared type and resource functions in a separate file in the
generated interface package. The bindings retain objects in a private table;
applications do not allocate representation integers or maintain resource maps.

For this WIT:

```wit
resource counter {
  constructor(start: u32);
  get: func() -> u32;
}
```

The generated requirements are:

```moonbit
declare type Counter
declare pub fn Counter::counter(start : UInt) -> Counter
declare pub fn Counter::get(self : Counter) -> UInt
declare fn Counter::dtor(self : Counter) -> Unit
```

An implementation is:

```moonbit
///|
struct Counter {
  value : UInt
}

///|
pub fn Counter::counter(start : UInt) -> Counter {
  { value: start }
}

///|
pub fn Counter::get(self : Counter) -> UInt {
  self.value
}

///|
fn Counter::dtor(_self : Counter) -> Unit {
  ()
}
```

The WIT constructor adapter registers the returned object and transfers the
resulting owned resource to its caller. A fallible constructor returns
`Result[Counter, E]`; only `Ok` is wrapped, and `Err` is passed through unchanged.
Constructing a `Counter` directly in
MoonBit only creates the application object. To create a locally owned component
resource, use `CounterHandle::counter(start)` or wrap an existing object with
`CounterHandle::new(counter)`.

## Methods and ownership

- WIT constructors, methods, and static functions are implemented on `Counter`.
  Generated counterparts on `CounterHandle` forward to those implementations.
  Thus `counter.get()` and `handle.get()` operate on the same object, while the
  latter first checks that the handle is still owned.
- An exported `own<counter>` parameter or result uses `CounterHandle`. The WIT
  constructor implementation is the exception: it returns `Counter`, which the
  bindings wrap automatically. Other functions returning an owned resource
  return a handle, preserving the ability to transfer an existing resource.
- An exported `borrow<counter>` parameter uses `Counter`, including inside
  records, variants, and lists. Resource methods receive this object as `self`.
- `handle.payload()` accesses the object of a locally owned resource.
- Transferring ownership invalidates the wrapper and its aliases. Calling a
  forwarded method, accessing the object, or transferring the same wrapper
  again traps. If the resource returns, the bindings create a new owning
  wrapper referring to the same stored object.
- `handle.drop()` releases a locally owned resource. Repeating it, or dropping
  a wrapper whose ownership was transferred, does nothing.
- Async forwarding methods accept the same background task group as their
  implementation methods.

`Counter` is an ordinary MoonBit value. The bindings do not enforce a borrow
lifetime on references to that value. Keeping an object reference does not
extend the component resource's lifetime or keep application resources open
after its destructor runs. Imported resources retain their existing ownership
behavior and constructor/method names.

## Async ownership and cleanup scopes

An async export's `background_group` lets explicitly spawned work outlive the
method's computation. The implementation
returns its result, the generated adapter calls CM `task.return`, and work in
the group can continue afterward. The MoonBit scope finishes when that work
finishes. Forwarding the group through a local handle method does not join it.
Work that must finish before producing the result belongs in the method itself
or in a separate local `with_task_group`.

There are three cleanup scopes:

- Use ordinary `defer` for values used only by the current computation.
- Use `background_group.add_defer` for an owned handle that must remain live
  until the group's children finish. Register it before the first operation
  that can fail or suspend.
- Give a returned future or stream producer ownership of the values it captures.
  Use `defer` inside the producer once it starts, and `on_unstarted_drop` to
  release captured handles if it is discarded before execution.

Shield asynchronous cleanup from cancellation with `protect_from_cancel`.
Group defer blocks must be `nocancel`; synchronous handle `drop()` already is.

For example, a response can transfer its owned body to the export's group:

```moonbit
pub async fn Response::collect(
  self : Response,
  background_group : @async-core.TaskGroup[Unit],
) -> (String, String?) {
  guard! self.body is Some(body)
  self.body = None
  background_group.add_defer(() => body.drop())
  body.collect(background_group)
}
```

The response no longer owns the body after clearing its field. The registered
cleanup retains the owning handle until all group children terminate, including
on error or cancellation. Neither an explicit `body.drop()` nor an ordinary
`defer body.drop()` here would cover work that outlives the method's return.
Do not wait for the background group inside the method: a worker may need the
caller to consume a returned stream or future before it can finish.

The body's collection code has a shorter scope. It takes its stream and trailer
future out of the object's fields before suspending, then uses deferred cleanup
to close both on success, error, or cancellation. The complete handwritten
example is in
[`resource-payloads/leaf.mbt`](../../../tests/runtime/moonbit/resource-payloads/leaf.mbt).
That body's implementation currently finishes its own work before returning;
the response demonstrates retaining ownership across the larger group scope.

Returned endpoint producers have a separate lifetime: the generated bridge
coroutines belong to the CM task and do not join the export's background group.
A group defer can therefore run before a returned producer finishes. Keep its
captured handle in the producer instead:

```moonbit
pub async fn deferred_get(
  counter : CounterHandle,
  _background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Future[UInt] {
  @async-core.Future::from(
    async fn() {
      defer counter.drop()
      counter.get()
    },
    on_unstarted_drop=() => counter.drop(),
  )
}
```

The unstarted callback runs only if the producer never starts; otherwise the
producer's `defer` handles completion, error, and cancellation. That callback
is synchronous, so it suits handles with synchronous `drop()`. Capturing an
endpoint whose cleanup must suspend requires a separate explicit ownership
and async-close design; it cannot simply be closed in that callback. Cleanup
of a resource *produced as the value* is also separate from cleanup of captures:
use `Future::ready_with_cleanup` or a stream's `cleanup` callback when needed.

An async WIT method still borrows its receiver; calling it does not transfer
ownership or automatically drop it. Keeping a borrowed payload in a closure
does not keep its resource alive. Background work must own the resources it
needs, or have an explicit owner that retains them through completion. The
[`exported-resource-lifecycle` test](../../../tests/runtime/moonbit/exported-resource-lifecycle/test.mbt)
demonstrates this with an owned handle: its caller receives the result, verifies
the resource is still live, then opens a gate so the worker can use it and the
group can destroy it. It does not retain a borrowed WIT argument after return.
It also covers a returned producer that remains live after the group finishes,
and local discard before producer execution. Local discard matters because
lowering an endpoint for return may already start its producer.

The body/response example deliberately asserts that collection has taken their
fields before destruction. This is a **test-only consumption requirement**,
not a general policy for abandoned responses. A production API must define
abandonment: for example, provide an async `close` operation that takes and
closes remaining endpoints under cancellation protection, then permit the
synchronous resource destructor to release the now-empty object. Both `collect`
and `close` must leave it safe to drop on success, error, or cancellation.
If callers may drop without closing, the destructor needs a synchronous
abandonment mechanism supported by the endpoint API. The fixture does not
provide that mechanism, and `background_group` is not a general finalizer for
arbitrary future drops.

## Visibility and generated names

Use a plain `struct Counter` (or another abstract type definition) to keep the
representation hidden from other MoonBit packages. The type name remains
accessible so other exported interfaces can accept a borrowed counter.
`pub struct` exposes its fields for reading, while `priv struct` is unsuitable
when the type appears in public signatures. The destructor can be a private
function because its ABI adapter lives in the same package.

Generated handle representations are abstract on both the exporting and
importing sides. Storage and resource intrinsics are private. Raw conversion
helpers needed by generated adapters in other packages are marked `#doc(hidden)`;
these public helpers are not application APIs. MoonBit package visibility does
not change the WIT interface exposed to other components.

WIT names take precedence over generated names. If `counter-handle` is already
a WIT type, the owning wrapper receives a numeric suffix. Likewise, if a WIT
method uses a helper name such as `drop`, `new`, or `payload`, the generated
helper receives a suffix and the WIT method keeps its original name. Consult the
generated declarations for the allocated names.

## Destruction

The generated destructor removes the object from storage before calling
`Counter::dtor`. The application destructor receives the actual object and can
release files, endpoints, or other resources it owns. Memory-only objects can
use an empty destructor.

Storage slots are reused through array indexing and a free-slot stack. Cleanup
may create or destroy other resources, including reusing the released slot.
Owned handles still require explicit transfer or `drop()`; MoonBit garbage
collection does not release a component resource automatically.

## Migrating integer-based implementations

This changes the generated MoonBit interface for exported resources. The WIT
interface and component ABI are unchanged.

1. Define the actual resource state under its original name, such as `Counter`.
2. Return that object from the WIT constructor implementation.
3. Implement borrowed methods directly on the object, replacing map lookups.
4. Use `CounterHandle` for other owned parameters and results. Create resources
   with `CounterHandle::new(object)` or `CounterHandle::counter(arguments)`.
5. Replace `value.rep()` and subsequent map lookups with `value.payload()`, or
   call a forwarded WIT method on the handle.
6. Keep `Counter::dtor` for application cleanup and remove map deletion and ID
   allocation logic.

Generated value derives are omitted for aggregates containing borrowed
implementation objects: those objects need not implement `Debug`, `Show`, or
`Eq`. The application can supply these implementations when needed.
