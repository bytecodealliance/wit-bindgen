//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn create_stream_with_values(count : UInt, out_stream : @async.OutStream[UInt]) -> Unit {
  let stream = out_stream.get_stream()
  for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
    let arr : Array[UInt] = [i.reinterpret_as_uint()]
    let _ = (stream.write)(arr[:])
  }
  (stream.close)()
}

///|
pub async fn create_unit_stream(count : UInt, out_stream : @async.OutStream[Unit]) -> Unit {
  let stream = out_stream.get_stream()
  for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
    let arr : Array[Unit] = [()]
    let _ = (stream.write)(arr[:])
  }
  (stream.close)()
}
