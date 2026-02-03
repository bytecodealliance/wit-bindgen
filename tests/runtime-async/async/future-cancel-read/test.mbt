//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn cancel_before_read(x : @async.Future[UInt], task_group : @async.TaskGroup[Unit]) -> Unit {
  x.drop()
}

///|
pub async fn cancel_after_read(x : @async.Future[UInt], task_group : @async.TaskGroup[Unit]) -> Unit {
  task_group.spawn_bg(async fn() { x.drop() })
  let _ = x.get() catch {
    @async.FutureReadError::Cancelled => return
    _ => panic()
  }
  panic()
}

///|
pub async fn start_read_then_cancel(
  data : @async.Future[UInt],
  signal : @async.Future[Unit],
  task_group : @async.TaskGroup[Unit],
) -> Unit {
  task_group.spawn_bg(async fn() {
    guard 4U == (try! data.get())
  })
  task_group.spawn_bg(async fn() {
    signal.get()
    data.drop()
  })
}
