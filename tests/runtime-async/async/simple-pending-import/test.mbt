//@ [lang]
//@ path = 'gen/interface/a/b/i/stub.mbt'

///|
pub async fn f(task_group : @async.TaskGroup[Unit]) -> Unit {
  for i = 0; i < 10; i = i + 1 {
    @async.pause()
  }
}
