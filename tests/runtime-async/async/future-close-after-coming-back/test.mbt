//@ [lang]
//@ path = 'gen/interface/a/b/theTest/stub.mbt'

pub async fn f(param : @async.FutureR[Unit], task_group : @async.TaskGroup[Unit]) -> @async.FutureR[Unit] {
  let out : @async.OutFuture[Unit] = @async.OutFuture::new()
  task_group.spawn_bg(async fn() {
    out.put(param.get())
  })
  wasmLiftF0(wasmLowerF0(out))
}
