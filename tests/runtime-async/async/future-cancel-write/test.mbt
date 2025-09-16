//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub fn take_then_drop(x : @ffi.FutureReader[String]) -> Unit {
  x.drop()
}

///|
pub async fn read_and_drop(
  task : @ffi.Task,
  x : @ffi.FutureReader[String],
) -> Unit noraise {
  task.spawn(x, fn() {
    let _ = x.read() catch { _ => raise @ffi.Cancelled::Cancelled }
  })
}
