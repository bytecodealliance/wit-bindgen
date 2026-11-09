//@ [lang]
//@ path = 'gen/interface/test/exported-resource-lifecycle/api/impl.mbt'

///|
struct Tracked {
  id : UInt
  reenter_on_drop : Bool
}

///|
struct BackgroundCheck {
  expected_id : UInt
}

///|
let dropped : Ref[Array[UInt]] = Ref([])

///|
let saved_alias : Ref[TrackedHandle?] = Ref(None)

///|
let reentrant_child : Ref[TrackedHandle?] = Ref(None)

///|
let producer_gate : Ref[@async-core.Promise[Unit]?] = Ref(None)

///|
pub fn Tracked::tracked(
  id : UInt,
  reenter_on_drop : Bool,
) -> Result[Tracked, String] {
  if id == 0 {
    Err("zero")
  } else {
    Ok({ id, reenter_on_drop, })
  }
}

///|
pub fn Tracked::id(self : Tracked) -> UInt {
  self.id
}

///|
fn Tracked::dtor(self : Tracked) -> Unit {
  dropped.val.push(self.id)
  if self.reenter_on_drop {
    // Keep the child alive after the outer destructor returns, so later cleanup
    // detects a reused storage slot being overwritten by the outer destruction.
    reentrant_child.val = Some(
      TrackedHandle::tracked(self.id + 1, false).unwrap(),
    )
  }
}

///|
pub fn retain_alias(value : TrackedHandle) -> TrackedHandle {
  saved_alias.val = Some(value)
  value
}

///|
pub fn drop_twice(value : TrackedHandle) -> Unit {
  let other = value
  value.drop()
  other.drop()
}

///|
pub fn drop_saved_alias() -> Unit {
  if saved_alias.val is Some(value) {
    value.drop()
    saved_alias.val = None
  }
}

///|
pub fn take_reentrant_child() -> TrackedHandle {
  let child = reentrant_child.val.unwrap()
  reentrant_child.val = None
  child
}

///|
pub fn take_dropped() -> FixedArray[UInt] {
  let result = dropped.val
  dropped.val = []
  FixedArray::from_array(result[:])
}

///|
pub async fn use_in_background(
  value : TrackedHandle,
  gate : @async-core.Future[Unit],
  background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Future[Unit] {
  let check = BackgroundCheckHandle::background_check(value.id())
  // The local forwarding method must return without joining its supplied group.
  // Its receiver can then be destroyed: the worker owns a different resource.
  defer check.drop()
  check.run(value, gate, background_group)
}

///|
pub fn BackgroundCheck::background_check(
  expected_id : UInt,
) -> BackgroundCheck {
  { expected_id, }
}

///|
fn BackgroundCheck::dtor(_self : BackgroundCheck) -> Unit {
  ()
}

///|
pub async fn BackgroundCheck::run(
  self : BackgroundCheck,
  value : TrackedHandle,
  gate : @async-core.Future[Unit],
  background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Future[Unit] {
  // Snapshot ordinary data from the borrowed receiver before returning. The
  // worker keeps an owned parameter; the group releases it after its children.
  let expected_id = self.expected_id
  let (completion, completed) = @async-core.Future::new()
  background_group.add_defer(() => {
    value.drop()
    // The caller may discard the notification without affecting cleanup.
    ignore(completed.complete(()))
  })
  background_group.spawn_bg(async fn() {
    defer @async-core.protect_from_cancel(() => gate.drop())
    gate.get()
    guard! value.id() == expected_id
  })
  // Do not join here: the caller can open the gate only after task.return.
  completion
}

///|
pub async fn use_in_producer(
  value : TrackedHandle,
  _background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Future[UInt] {
  // A local unit-valued gate controls the test without capturing another CM
  // endpoint. Only the owned resource needs cleanup if the producer never runs.
  let (gate, release) = @async-core.Future::new()
  producer_gate.val = Some(release)
  @async-core.Future::from(
    async fn() {
      defer value.drop()
      defer @async-core.protect_from_cancel(() => gate.drop())
      gate.get()
      value.id()
    },
    on_unstarted_drop=() => value.drop(),
  )
}

///|
pub fn release_producer() -> Unit {
  let release = producer_gate.val.unwrap()
  producer_gate.val = None
  guard! release.complete(())
}

///|
pub async fn discard_unstarted_producer(
  value : TrackedHandle,
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  // Drop locally: lowering a returned endpoint may already start its producer.
  let future = @async-core.Future::from(
    async fn() -> UInt {
      defer value.drop()
      abort("discarded producer must not run")
    },
    on_unstarted_drop=() => value.drop(),
  )
  future.drop()
}
