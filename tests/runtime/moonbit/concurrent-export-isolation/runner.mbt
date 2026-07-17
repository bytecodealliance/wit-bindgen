//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": [{ "path": "test/moonbit-concurrent-export-isolation/async-core", "alias": "async-core" }, { "path": "test/moonbit-concurrent-export-isolation/interface/test/moonbit-concurrent-export-isolation/operations", "alias": "operations" }] }"""

///|
pub async fn run(background_group : @async-core.TaskGroup[Unit]) -> Unit {
  let isolated = background_group.spawn(
    async fn() -> Bool { @operations.check_context_isolation() },
    allow_failure=true,
  )
  let isolated_again = background_group.spawn(
    async fn() -> Bool { @operations.check_context_isolation() },
    allow_failure=true,
  )
  let completed = background_group.spawn(
    async fn() -> UInt { @operations.complete_now() },
    allow_failure=true,
  )
  guard isolated.wait() else { panic() }
  guard isolated_again.wait() else { panic() }
  guard completed.wait() == 42U else { panic() }

  let (gate, gate_sink) = @async-core.Stream::new(capacity=0)
  let waiting = background_group.spawn(
    async fn() -> Unit {
      @operations.wait_for_gate(
        @async-core.Future::from(async fn() {
          guard gate.read(1) is Some(signal) && signal.length() == 1
        }),
      )
    },
    allow_failure=true,
  )
  let completing = background_group.spawn(
    async fn() -> UInt {
      let result = @operations.complete_now()
      let signal : FixedArray[Unit] = [()]
      guard gate_sink.write_all(signal[:]) else { panic() }
      gate_sink.close()
      result
    },
    allow_failure=true,
  )
  guard completing.wait() == 42U else { panic() }
  waiting.wait()

  let (release_stream, release_sink) = @async-core.Stream::new(capacity=0)
  let held_stream = @operations.hold_stream(
    @async-core.Future::from(async fn() {
      guard release_stream.read(1) is Some(signal) && signal.length() == 1 else {
        abort("stream release signal closed early")
      }
    }),
  )
  let completing_while_stream_is_held = background_group.spawn(
    async fn() -> UInt { @operations.complete_now() },
    allow_failure=true,
  )

  // Give the imported call several scheduler turns without releasing the
  // stream producer. Component backpressure must not be tied to bridge life.
  let (turns, turn_sink) = @async-core.Stream::new(capacity=0)
  let turn_writer = background_group.spawn(
    async fn() -> Bool {
      let turn : FixedArray[Unit] = [()]
      for _ in 0..<32 {
        guard turn_sink.write_all(turn[:]) else { return false }
      }
      turn_sink.close()
      true
    },
    allow_failure=true,
  )
  for _ in 0..<32 {
    guard turns.read(1) is Some(turn) && turn.length() == 1 else { panic() }
  }
  guard turn_writer.wait() else { panic() }
  let completed_before_release = completing_while_stream_is_held.try_wait() ==
    Some(42U)

  let release : FixedArray[Unit] = [()]
  guard release_sink.write_all(release[:]) else { panic() }
  release_sink.close()
  guard held_stream.read(1) is Some(chunk) &&
    chunk.length() == 1 &&
    chunk[0] == b'x' else {
    panic()
  }
  guard held_stream.read(1) is None else { panic() }
  guard completing_while_stream_is_held.wait() == 42U else { panic() }
  guard completed_before_release else { panic() }

  let (cross_future, cross_promise) = @async-core.Future::new()
  @operations.begin_cross_task_future_read(cross_future)
  for _ in 0..<32 {
    if @operations.cross_task_future_read_state() >= 1 {
      break
    }
    ignore(@operations.complete_now())
  }
  guard @operations.cross_task_future_read_state() == 1 else { panic() }
  @operations.drop_cross_task_future_read()
  guard cross_promise.complete(()) else { panic() }
  for _ in 0..<32 {
    if @operations.cross_task_future_read_state() == 2 {
      break
    }
    ignore(@operations.complete_now())
  }
  guard @operations.cross_task_future_read_state() == 2 else { panic() }

  let (cross_stream, cross_sink) = @async-core.Stream::new(capacity=0)
  @operations.begin_cross_task_stream_read(cross_stream)
  for _ in 0..<32 {
    if @operations.cross_task_stream_read_state() >= 1 {
      break
    }
    ignore(@operations.complete_now())
  }
  guard @operations.cross_task_stream_read_state() == 1 else { panic() }
  @operations.drop_cross_task_stream_read()
  let discarded : FixedArray[Byte] = [7]
  guard cross_sink.write_all(discarded[:]) else { panic() }
  cross_sink.close()
  for _ in 0..<32 {
    if @operations.cross_task_stream_read_state() == 2 {
      break
    }
    ignore(@operations.complete_now())
  }
  guard @operations.cross_task_stream_read_state() == 2 else { panic() }
}
