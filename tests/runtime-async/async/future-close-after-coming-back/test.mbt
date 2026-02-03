//@ [lang]
//@ path = 'gen/interface/a/b/theTest/stub.mbt'

pub async fn f(param : @async.Future[Unit], task_group : @async.TaskGroup[Unit]) -> @async.Future[Unit] {
  @async.Future::from(async fn() { param.get() })
}
