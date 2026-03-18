//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn take_then_drop(x : @async.Future[String], task_group : @async.TaskGroup[Unit]) -> Unit {
  x.drop()
}

///|
pub async fn read_and_drop(x : @async.Future[String], task_group : @async.TaskGroup[Unit]) -> Unit {
  let _ = x.get()
}
