//@ [lang]
//@ path = 'gen/interface/test_/common/iMiddle/stub.mbt'

///|
pub async fn f(task_group : @async.TaskGroup[Unit]) -> Unit {
  @async.pause()
  @async.pause()
}
