//@ [lang]
//@ path = 'gen/interface/my/test/i/stub.mbt'

///|
pub async fn cancel_before_read(
  x : @async-core.Future[UInt],
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  x.drop()
}

///|
pub async fn cancel_after_read(
  x : @async-core.Future[UInt],
  background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  let started = @async-core.Semaphore(1, initial_value=0)
  let reader = background_group.spawn(async fn() -> Unit {
    started.release()
    let _ = x.get() catch {
      @async-core.FutureReadError::Dropped => return
      err => raise err
    }
    panic()
  })
  // Let the reader suspend before dropping the endpoint.
  started.acquire()
  x.drop()
  reader.wait()
}

///|
pub async fn start_read_then_cancel(
  data : @async-core.Future[UInt],
  signal : @async-core.Future[Unit],
  background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  background_group.spawn_bg(async fn() { let _ = data.get() })
  background_group.spawn_bg(async fn() { signal.get() })
}
