//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub fn create_stream_with_values(count : UInt) -> @async.OutStream[UInt] {
  let out : @async.OutStream[UInt] = { stream: None, coroutine: None }
  // The stream writing will happen when get_stream is called
  // For now, we set up the stream and spawn a task to write values
  let _ = @async.spawn(async fn() {
    let stream = out.get_stream()
    for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
      let arr : Array[UInt] = [i.reinterpret_as_uint()]
      let _ = (stream.write)(arr[:])
    }
    (stream.close)()
  })
  out
}

///|
pub fn create_unit_stream(count : UInt) -> @async.OutStream[Unit] {
  let out : @async.OutStream[Unit] = { stream: None, coroutine: None }
  let _ = @async.spawn(async fn() {
    let stream = out.get_stream()
    for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
      let arr : Array[Unit] = [()]
      let _ = (stream.write)(arr[:])
    }
    (stream.close)()
  })
  out
}
