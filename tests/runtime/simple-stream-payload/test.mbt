//@ [lang] 
//@ path = 'gen/interface/my/test_/i/stub.mbt'

pub async fn read_stream(x : @ffi.StreamReader[Byte]) -> Unit raise {
      let task = @ffi.current_task()
      let buffer = FixedArray::make(10, Byte::default());

      task.wait(fn(){
            let _ = x.read(buffer, 1) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
      task.wait(fn(){
            let _ = x.read(buffer, 2, offset=1) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
      task.wait(fn(){
            let _ = x.read(buffer, 1, offset=3) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
      task.wait(fn(){
            let _ = x.read(buffer, 1, offset=4) catch { _ => raise @ffi.Cancelled::Cancelled }
      })
}
