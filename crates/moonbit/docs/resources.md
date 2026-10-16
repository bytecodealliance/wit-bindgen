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
