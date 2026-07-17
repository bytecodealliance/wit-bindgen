//@ [lang]
//@ path = 'gen/interface/test/moonbit-concurrent-export-isolation/operations/stub.mbt'

///|
let cross_task_future : Ref[@async-core.Future[Unit]?] = Ref(None)

///|
let cross_task_future_state : Ref[Byte] = Ref(0)

///|
let cross_task_stream : Ref[@async-core.Stream[Byte]?] = Ref(None)

///|
let cross_task_stream_state : Ref[Byte] = Ref(0)

///|
pub async fn check_context_isolation(
  background_group : @async-core.TaskGroup[Unit],
) -> Bool {
  let (turns, turn_sink) = @async-core.Stream::new(capacity=0)
  background_group.spawn_bg(async fn() {
    let turn : FixedArray[Unit] = [()]
    for _ in 0..<2048 {
      guard turn_sink.write_all(turn[:]) else { return }
    }
    turn_sink.close()
  })
  // Exceed one scheduler callback budget with real suspend/resume cycles so
  // another export starts while this coroutine is still in flight.
  for _ in 0..<2048 {
    guard turns.read(1) is Some(turn) && turn.length() == 1 else {
      return false
    }
  }
  true
}

///|
pub async fn wait_for_gate(
  gate : @async-core.Future[Unit],
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  let _ = gate.get()
}

///|
pub async fn hold_stream(
  gate : @async-core.Future[Unit],
  _background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Stream[Byte] {
  @async-core.Stream::produce(async fn(sink) {
    let _ = gate.get()
    guard sink.write_all_bytes(b"x"[:]) else { return }
    sink.close()
  })
}

///|
pub async fn complete_now(
  _background_group : @async-core.TaskGroup[Unit],
) -> UInt {
  42
}

///|
pub async fn begin_cross_task_future_read(
  value : @async-core.Future[Unit],
  background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  cross_task_future.val = Some(value)
  cross_task_future_state.val = 0
  background_group.spawn_bg(async fn() {
    cross_task_future_state.val = 1
    match cross_task_future.val {
      Some(value) => value.get() catch { _ => () }
      None => panic()
    }
    cross_task_future_state.val = 2
  })
}

///|
pub fn cross_task_future_read_state() -> Byte {
  cross_task_future_state.val
}

///|
pub async fn drop_cross_task_future_read(
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  match cross_task_future.val {
    Some(value) => value.drop()
    None => panic()
  }
}

///|
pub async fn begin_cross_task_stream_read(
  value : @async-core.Stream[Byte],
  background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  cross_task_stream.val = Some(value)
  cross_task_stream_state.val = 0
  background_group.spawn_bg(async fn() {
    cross_task_stream_state.val = 1
    match cross_task_stream.val {
      Some(value) => ignore(value.read(1)) catch { _ => () }
      None => panic()
    }
    cross_task_stream_state.val = 2
  })
}

///|
pub fn cross_task_stream_read_state() -> Byte {
  cross_task_stream_state.val
}

///|
pub async fn drop_cross_task_stream_read(
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  match cross_task_stream.val {
    Some(value) => value.drop()
    None => panic()
  }
}
