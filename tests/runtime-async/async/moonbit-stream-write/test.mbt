//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn create_stream_with_values(count : UInt, task_group : @async.TaskGroup[Unit]) -> @async.OutStream[UInt] {
  let out : @async.OutStream[UInt] = { stream: None, coroutine: None }
  // The stream writing will happen when get_stream is called
  // Spawn a background task to write values using the task group
  task_group.spawn_bg(async fn() {
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
pub async fn create_unit_stream(count : UInt, task_group : @async.TaskGroup[Unit]) -> @async.OutStream[Unit] {
  let out : @async.OutStream[Unit] = { stream: None, coroutine: None }
  task_group.spawn_bg(async fn() {
    let stream = out.get_stream()
    for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
      let arr : Array[Unit] = [()]
      let _ = (stream.write)(arr[:])
    }
    (stream.close)()
  })
  out
}
