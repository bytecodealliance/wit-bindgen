//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub fn take_then_drop(x : @ffi.FutureReader[String]) -> Unit {
  let _ = x.drop()
}

///|
pub async fn read_and_drop(
  x : @ffi.FutureReader[String],
) -> Unit noraise {
  let task = @ffi.current_task()
  let _ = task.spawn(fn() {
    let _ = x.read() catch { _ => raise @ffi.Cancelled::Cancelled }
  })
}
