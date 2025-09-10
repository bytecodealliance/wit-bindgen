//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'


///|
pub async fn read_future(task: @ffi.Task, x : @ffi.Future[Unit]) -> Unit raise {
  task.add_defer(() => {x.drop_readable()})
  let _ = x.read()
}

///|
pub async fn drop_future(task: @ffi.Task, x : @ffi.Future[Unit]) -> Unit noraise {
  defer x.drop_readable()
}
