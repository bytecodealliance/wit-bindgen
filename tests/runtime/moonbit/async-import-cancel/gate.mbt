//@ [lang]
//@ path = 'gen/interface/test/async-import-cancel/gate-control/stub.mbt'

///|
let return_started : Ref[Bool] = Ref(false)

///|
let return_released : Ref[Bool] = Ref(false)

///|
pub fn mark_started() -> Unit {
  return_started.val = true
}

///|
pub fn started() -> Bool {
  return_started.val
}

///|
pub fn release() -> Unit {
  return_released.val = true
}

///|
pub fn released() -> Bool {
  return_released.val
}
