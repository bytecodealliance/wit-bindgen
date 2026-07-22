//@ [lang]
//@ path = 'gen/interface/my/test/i/stub.mbt'

///|
pub async fn read_future(
  x : @async-core.Future[Unit],
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  let _ = x.get() catch { _ => () }
}

///|
pub async fn drop_future(
  x : @async-core.Future[Unit],
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  x.drop()
}
