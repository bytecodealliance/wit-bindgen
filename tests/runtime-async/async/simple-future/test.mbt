//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'


///|
pub async fn async_read_future(task: @ffi.WaitableTask, x : @ffi.Future[Unit]) -> Unit raise {
  task.add_defer(() => {x.drop_readable()})
  let _ = x.read()
}

///|
pub async fn async_drop_future(task: @ffi.WaitableTask, x : @ffi.Future[Unit]) -> Unit noraise {
  defer x.drop_readable()
}
