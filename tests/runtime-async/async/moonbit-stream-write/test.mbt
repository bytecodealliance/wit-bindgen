//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
pub async fn create_stream_with_values(count : UInt, task_group : @async.TaskGroup[Unit]) -> @async.StreamR[UInt] {
  let out : @async.OutStream[UInt] = @async.OutStream::new()
  task_group.spawn_bg(async fn() {
    let stream_w = out.get_stream()
    for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
      let arr : Array[UInt] = [i.reinterpret_as_uint()]
      let _ = (stream_w.write)(arr[:])
    }
    (stream_w.close)()
  })
  wasmLiftCreateStreamWithValues0(wasmLowerCreateStreamWithValues0(out))
}

///|
pub async fn create_unit_stream(count : UInt, task_group : @async.TaskGroup[Unit]) -> @async.StreamR[Unit] {
  let out : @async.OutStream[Unit] = @async.OutStream::new()
  task_group.spawn_bg(async fn() {
    let stream_w = out.get_stream()
    for i = 0; i < count.reinterpret_as_int(); i = i + 1 {
      let arr : Array[Unit] = [()]
      let _ = (stream_w.write)(arr[:])
    }
    (stream_w.close)()
  })
  wasmLiftCreateUnitStream0(wasmLowerCreateUnitStream0(out))
}
