//@ [lang]
//@ path = 'gen/interface/a/b/theTest/stub.mbt'

///|
pub async fn f(param : @async.Future[Unit], task_group : @async.TaskGroup[Unit]) -> Unit {
  param.get()
}
