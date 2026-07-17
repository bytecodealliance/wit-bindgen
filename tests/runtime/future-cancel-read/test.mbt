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
  background_group.spawn_bg(
    async fn() {
      let _ = x.get() catch { _ => raise @async-core.Cancelled::Cancelled }
    }
  )
  x.drop()
}

///|
pub async fn start_read_then_cancel(
  data : @async-core.Future[UInt],
  signal : @async-core.Future[Unit],
  background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  background_group.spawn_bg(
    async fn() { let _ = data.get() catch { _ => raise @async-core.Cancelled::Cancelled } },
  )
  background_group.spawn_bg(
    async fn() { signal.get() catch { _ => raise @async-core.Cancelled::Cancelled } },
  )
}
