//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub fn take_then_drop(x : @ffi.FutureReader[String]) -> Unit {
  defer x.drop()
}


///|
pub async fn read_and_drop(task: @ffi.Task, x : @ffi.FutureReader[String]) -> Unit noraise {
  let _ = x.read_then_drop() catch {
    _ => panic()
  }
}


