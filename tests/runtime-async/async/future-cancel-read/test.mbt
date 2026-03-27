//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn cancel_before_read(
  x : @ffi.FutureReader[UInt],
) -> Unit noraise {
  let task = @ffi.current_task()
  let _ = x.drop()
}

///|
pub async fn cancel_after_read(
  x : @ffi.FutureReader[UInt],
) -> Unit noraise {
  let task = @ffi.current_task()
  task.spawn(
    fn() {
      let _ = x.read() catch { _ => raise @ffi.Cancelled::Cancelled }
    }
  )
  task.cancel_waitable(x)
}

///|
pub async fn start_read_then_cancel(
  data : @ffi.FutureReader[UInt],
  signal : @ffi.FutureReader[Unit],
) -> Unit noraise {
  let task = @ffi.current_task()
  task.spawn(
    fn() { let _ = data.read() catch { _ => raise @ffi.Cancelled::Cancelled } },
  ) 
  task.spawn(
    fn() { signal.read() catch { _ => raise @ffi.Cancelled::Cancelled } },
  )
}

