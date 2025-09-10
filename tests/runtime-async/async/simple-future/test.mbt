//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'


///|
pub async fn read_future(task: @ffi.Task, x : @ffi.FutureReader[Unit]) -> Unit raise {
  task.add_defer(() => {x.drop()})
  let _ = x.read()
}

///|
pub async fn drop_future(task: @ffi.Task, x : @ffi.FutureReader[Unit]) -> Unit noraise {
  defer x.drop()
}
