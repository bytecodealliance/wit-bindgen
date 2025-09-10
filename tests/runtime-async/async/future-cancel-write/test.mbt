//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub fn take_then_drop(x : @ffi.Future[String]) -> Unit {
  defer x.drop_readable()
}


///|
pub async fn read_and_drop(task: @ffi.Task, x : @ffi.Future[String]) -> Unit noraise {
  let _ = x.read_then_drop() catch {
    _ => panic()
  }
}


