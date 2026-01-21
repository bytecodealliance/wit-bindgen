//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub fn create_future_with_value(value : UInt) -> @async.OutFuture[UInt] {
  let out : @async.OutFuture[UInt] = { value: None, coroutine: None }
  out.put(value)
  out
}

///|
pub fn create_unit_future() -> @async.OutFuture[Unit] {
  let out : @async.OutFuture[Unit] = { value: None, coroutine: None }
  out.put(())
  out
}
