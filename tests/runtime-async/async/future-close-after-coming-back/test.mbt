//@ [lang]
//@ path = 'gen/interface/a/b/theTest/stub.mbt'

pub async fn f(param : @async.FutureR[Unit], task_group : @async.TaskGroup[Unit]) -> @async.OutFuture[Unit] {
  let out : @async.OutFuture[Unit] = { ..Default::default() }
  task_group.spawn_bg(async fn() {
    out.put(param.get())
  })
  out
}

