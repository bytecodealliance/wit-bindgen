//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn ping(
  x : @async.Future[String],
  y : String,
  task_group : @async.TaskGroup[Unit],
) -> @async.Future[String] {
  let message = x.get() + y
  @async.Future::ready(message)
}

///|
pub async fn pong(x : @async.Future[String], task_group : @async.TaskGroup[Unit]) -> String {
  x.get()
}
