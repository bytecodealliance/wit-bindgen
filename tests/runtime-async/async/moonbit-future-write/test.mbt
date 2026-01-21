//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn create_future_with_value(value : UInt, task_group : @async.TaskGroup[@async.OutFuture[UInt]]) -> @async.OutFuture[UInt] {
  let out : @async.OutFuture[UInt] = { value: None, coroutine: None }
  out.put(value)
  out
}

///|
pub async fn create_unit_future(task_group : @async.TaskGroup[@async.OutFuture[Unit]]) -> @async.OutFuture[Unit] {
  let out : @async.OutFuture[Unit] = { value: None, coroutine: None }
  out.put(())
  out
}
