//@ [lang]
//@ path = 'gen/interface/my/test/i/stub.mbt'

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
    @async-core.backpressure_inc()
  } else {
    @async-core.backpressure_dec()
  }
}
