//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn ping(
  x : @async.CMFuture[String],
  y : String,
  task_group : @async.TaskGroup[Unit],
) -> @async.CMFuture[String] {
  let message = x.get() + y
  @async.CMFuture::from(async fn() { message })
}

///|
pub async fn pong(x : @async.CMFuture[String], task_group : @async.TaskGroup[Unit]) -> String {
  x.get()
}
