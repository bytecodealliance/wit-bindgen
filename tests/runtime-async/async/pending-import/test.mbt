//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn pending_import(x : @async.Future[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  x.get()
}
