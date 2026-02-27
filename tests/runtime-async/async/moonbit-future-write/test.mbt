//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn create_future_with_value(value : UInt, task_group : @async.TaskGroup[Unit]) -> @async.CMFuture[UInt] {
  @async.CMFuture::ready(value)
}

///|
pub async fn create_unit_future(task_group : @async.TaskGroup[Unit]) -> @async.CMFuture[Unit] {
  @async.CMFuture::ready(())
}

///|
pub async fn create_nested_future(value : UInt, task_group : @async.TaskGroup[Unit]) -> @async.CMFuture[@async.CMFuture[UInt]] {
  @async.CMFuture::ready(@async.CMFuture::ready(value))
}

///|
pub async fn create_nested_future_record(value : UInt, task_group : @async.TaskGroup[Unit]) -> NestedFutureRecord {
  { nested: @async.CMFuture::ready(@async.CMFuture::ready(value)) }
}
