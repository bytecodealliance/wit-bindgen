//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn pending_import(x : @async.CMFuture[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  x.get()
}

///|
pub fn backpressure_set(x : Bool) -> Unit {
  if x {
    @async.backpressure_inc()
  } else {
    @async.backpressure_dec()
  }
}
