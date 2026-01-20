//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn create_future_with_value(value : UInt, out_future : @async.OutFuture[UInt]) -> Unit {
  out_future.put(value)
}

///|
pub async fn create_unit_future(out_future : @async.OutFuture[Unit]) -> Unit {
  out_future.put(())
}
