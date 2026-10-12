//@ [lang]
//@ path = 'gen/interface/test/exported-resource-lifecycle/api/impl.mbt'

///|
struct Tracked {
  id : UInt
  reenter_on_drop : Bool
}

///|
let dropped : Ref[Array[UInt]] = Ref([])

///|
let saved_alias : Ref[TrackedHandle?] = Ref(None)

///|
let reentrant_child : Ref[TrackedHandle?] = Ref(None)

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
