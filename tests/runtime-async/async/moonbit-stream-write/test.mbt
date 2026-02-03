//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn create_stream_with_values(count : UInt, task_group : @async.TaskGroup[Unit]) -> @async.Stream[UInt] {
  @async.Stream::from(async fn(sink : @async.Sink[UInt]) {
    for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
      let arr : Array[UInt] = [i.reinterpret_as_uint()]
      let _ = sink.write(arr[:])
    }
    sink.close()
  })
}

///|
pub async fn create_unit_stream(count : UInt, task_group : @async.TaskGroup[Unit]) -> @async.Stream[Unit] {
  @async.Stream::from(async fn(sink : @async.Sink[Unit]) {
    for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
      let arr : Array[Unit] = [()]
      let _ = sink.write(arr[:])
    }
    sink.close()
  })
}
