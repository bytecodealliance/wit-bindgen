//@ [lang] 
//@ path = 'gen/interface/my/test_/i/stub.mbt'

pub async fn read_stream(task: @ffi.Task, x : @ffi.StreamReader[Byte]) -> Unit raise {
      let buffer = FixedArray::make(10, Byte::default());

      task.wait(x, fn(){
            let _ = x.read(buffer, 1) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
      task.wait(x, fn(){
            let _ = x.read(buffer, 2, offset=1) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
      task.wait(x, fn(){
            let _ = x.read(buffer, 1, offset=3) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
      task.wait(x, fn(){
            let _ = x.read(buffer, 1, offset=4) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
}
