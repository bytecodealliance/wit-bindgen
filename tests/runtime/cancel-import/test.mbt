//@ [lang]
//@ path = 'gen/interface/my/test/i/stub.mbt'

///|
fn wasmBackpressureInc() = "$root" "[backpressure-inc]"

///|
fn wasmBackpressureDec() = "$root" "[backpressure-dec]"

///|
pub async fn pending_import(
  x : @async-core.Future[Unit],
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  x.get()
}

///|
pub fn backpressure_set(x : Bool) -> Unit {
  if x {
    wasmBackpressureInc()
  } else {
    wasmBackpressureDec()
  }
}
