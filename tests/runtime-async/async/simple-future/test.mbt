//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn read_future(
  task : @ffi.Task,
  x : @ffi.FutureReader[Unit],
) -> Unit noraise {
  task.spawn(x, fn() {
    let _ = x.read() catch { _ => raise @ffi.Cancelled::Cancelled }

  })
}

///|
pub async fn drop_future(
  task : @ffi.Task,
  x : @ffi.FutureReader[Unit],
) -> Unit noraise {
  task.spawn(x, fn() {
  })
}
