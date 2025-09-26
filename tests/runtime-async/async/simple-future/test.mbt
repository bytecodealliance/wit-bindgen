//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn read_future(
  x : @ffi.FutureReader[Unit],
) -> Unit noraise {
  let task = @ffi.current_task()
  task.spawn(fn() {
    let _ = x.read() catch { _ => raise @ffi.Cancelled::Cancelled }

  })
}

///|
pub async fn drop_future(
  x : @ffi.FutureReader[Unit],
) -> Unit noraise {
  let task = @ffi.current_task()
  let _ = x.drop()
}
