//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn read_future(x : @async.Future[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  x.get()
}

///|
pub async fn drop_future(x : @async.Future[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  x.drop()
}
