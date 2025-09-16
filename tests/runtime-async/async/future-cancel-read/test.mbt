//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn cancel_before_read(
  task : @ffi.Task,
  x : @ffi.FutureReader[UInt],
) -> Unit noraise {
  task.spawn(x, fn() {})
}

///|
pub async fn cancel_after_read(
  task : @ffi.Task,
  x : @ffi.FutureReader[UInt],
) -> Unit noraise {
  task.spawn(
    x, fn() {
      let _ = x.read() catch { _ => raise @ffi.Cancelled::Cancelled }
    }
  )
  task.cancel_waitable(x)
}

///|
pub async fn start_read_then_cancel(
  task : @ffi.Task,
  data : @ffi.FutureReader[UInt],
  signal : @ffi.FutureReader[Unit],
) -> Unit noraise {
  task.spawn(
    data,
    fn() { let _ = data.read() catch { _ => raise @ffi.Cancelled::Cancelled } },
  ) 
  task.spawn(
    signal,
    fn() { signal.read() catch { _ => raise @ffi.Cancelled::Cancelled } },
  )
  // task.add_defer(fn() {data.drop(); signal.drop()})
}

