//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn create_future_with_value(value : UInt, task_group : @async.TaskGroup[Unit]) -> @async.Future[UInt] {
  @async.Future::ready(value)
}

///|
pub async fn create_unit_future(task_group : @async.TaskGroup[Unit]) -> @async.Future[Unit] {
  @async.Future::ready(())
}

///|
pub async fn create_nested_future(value : UInt, task_group : @async.TaskGroup[Unit]) -> @async.Future[@async.Future[UInt]] {
  let inner = @async.Future::ready(value)
  @async.Future::ready(inner)
}

///|
pub async fn create_nested_future_record(value : UInt, task_group : @async.TaskGroup[Unit]) -> NestedFutureRecord {
  let inner = @async.Future::ready(value)
  { nested: @async.Future::ready(inner) }
}
