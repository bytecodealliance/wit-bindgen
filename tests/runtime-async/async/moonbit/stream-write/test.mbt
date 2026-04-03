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

///|
pub async fn create_bridge_stream_with_signal(
  signal : @async.Future[Unit],
  task_group : @async.TaskGroup[Unit],
) -> (@async.Stream[UInt], @async.Future[Bool]) {
  let (stream, sink) = @async.Stream::new(capacity=1)
  let (done, promise) = @async.Future::new()
  task_group.spawn_bg(async fn() {
    let first : Array[UInt] = [0U]
    let rest : Array[UInt] = [1U, 2U, 3U]
    assert_eq(sink.write(first[:]), 1)
    signal.get()
    promise.write(sink.write_all(rest[:]))
    sink.close()
  })
  (stream, done)
}

///|
pub async fn create_bridge_unit_stream_with_signal(
  signal : @async.Future[Unit],
  task_group : @async.TaskGroup[Unit],
) -> (@async.Stream[Unit], @async.Future[Bool]) {
  let (stream, sink) = @async.Stream::new(capacity=1)
  let (done, promise) = @async.Future::new()
  task_group.spawn_bg(async fn() {
    let first : Array[Unit] = [()]
    let rest : Array[Unit] = [(), (), ()]
    assert_eq(sink.write(first[:]), 1)
    signal.get()
    promise.write(sink.write_all(rest[:]))
    sink.close()
  })
  (stream, done)
}
