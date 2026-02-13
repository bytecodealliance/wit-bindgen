//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn pending_import(x : @async.Future[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  let _ = @async.protect_from_cancel(async fn() {
    x.get()
  }) catch {
    @async.FutureReadError::Cancelled => {
      return
    }
    @async.Cancelled::Cancelled => {
      return
    }
    _ => panic()
  }
}

///|
pub fn backpressure_set(x : Bool) -> Unit {
  if x {
    @async.backpressure_inc()
  } else {
    @async.backpressure_dec()
  }
}
