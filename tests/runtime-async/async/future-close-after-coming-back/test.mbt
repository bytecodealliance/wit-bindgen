//@ [lang]
//@ path = 'gen/interface/a/b/theTest/stub.mbt'

pub async fn f(param : @async.CMFuture[Unit], task_group : @async.TaskGroup[Unit]) -> @async.CMFuture[Unit] {
  @async.CMFuture::from(async fn() { param.get() })
}
