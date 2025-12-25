//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn cancel_before_read(x : @async.FutureR[UInt]) -> Unit {
  x.drop()
}

///|
pub async fn cancel_after_read(x : @async.FutureR[UInt]) -> Unit {
  @async.with_task_group(tg => {
    tg.spawn_bg(() => x.drop())
    let _ = x.get() catch {
      @async.FutureReadError::Cancelled => return
      _ => panic()
    }
    panic()
  })
}

///|
pub async fn start_read_then_cancel(
  data : @async.FutureR[UInt],
  signal : @async.FutureR[Unit],
) -> Unit {
  @async.with_task_group(tg => {
    tg.spawn_bg(() => {
      guard 4U == (try! data.get())
    })
    tg.spawn_bg(() => {
      signal.get()
      data.drop()
    })
  })
}
