//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn create_future_with_value(value : UInt, task_group : @async.TaskGroup[Unit]) -> @async.FutureR[UInt] {
  let out : @async.OutFuture[UInt] = @async.OutFuture::new()
  out.put(value)
  wasmLiftCreateFutureWithValue0(wasmLowerCreateFutureWithValue0(out))
}

///|
pub async fn create_unit_future(task_group : @async.TaskGroup[Unit]) -> @async.FutureR[Unit] {
  let out : @async.OutFuture[Unit] = @async.OutFuture::new()
  out.put(())
  wasmLiftCreateUnitFuture0(wasmLowerCreateUnitFuture0(out))
}
