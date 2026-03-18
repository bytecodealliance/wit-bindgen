//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn create_stream_with_values(count : UInt, task_group : @async.TaskGroup[Unit]) -> @async.Stream[UInt] {
  @async.Stream::from(async fn(sink : @async.Sink[UInt]) {
    let arr : Array[UInt] = []
    for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
      arr.push(i.reinterpret_as_uint())
    }
    assert_true(sink.write_all(arr[:]))
    sink.close()
  })
}

///|
pub async fn create_unit_stream(count : UInt, task_group : @async.TaskGroup[Unit]) -> @async.Stream[Unit] {
  @async.Stream::from(async fn(sink : @async.Sink[Unit]) {
    let arr : Array[Unit] = []
    for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
      arr.push(())
    }
    assert_true(sink.write_all(arr[:]))
    sink.close()
  })
}
