//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

pub async fn read_stream(task: @ffi.Task, x : @ffi.StreamReader[Unit]) -> Unit noraise {
      let buffer = FixedArray::make(10, Unit::default());

      task.spawn(x, fn(){
            let _ = x.read(buffer, 1) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
      task.spawn(x, fn(){
            let _ = x.read(buffer, 2) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
}

