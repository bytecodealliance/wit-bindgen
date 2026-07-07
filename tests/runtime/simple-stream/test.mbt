//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

pub async fn read_stream(x : @ffi.StreamReader[Unit]) -> Unit noraise {
      let task = @ffi.current_task()
      let buffer = FixedArray::make(10, Unit::default());

      task.spawn(fn(){
            let _ = x.read(buffer, 1) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
      task.spawn(fn(){
            let _ = x.read(buffer, 2) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
}

