//@ [lang]
//@ path = 'gen/interface/my/test/i/stub.mbt'

///|
pub fn take_then_drop(x : @async-core.Future[String]) -> Unit {
  x.drop_sync()
}

///|
pub async fn read_and_drop(
  x : @async-core.Future[String],
  background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  background_group.spawn_bg(async fn() {
    let _ = x.get() catch { _ => raise @async-core.Cancelled::Cancelled }
  })
}
