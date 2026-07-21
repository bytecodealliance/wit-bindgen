//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "supported-targets": "+wasm", "import": [{ "path": "test/async-import-cancel/async-core", "alias": "async-core" }, { "path": "test/async-import-cancel/interface/test/async-import-cancel/pending", "alias": "pending" }, { "path": "test/async-import-cancel/interface/test/async-import-cancel/gate-control", "alias": "gate-control" }] }"""

///|
async fn settle_leaf_counts(expected_drops : UInt) -> Unit {
  for _ in 0..<32 {
    if @pending.leaf_live_count() == 0U &&
      @pending.leaf_drop_count() == expected_drops {
      break
    }
    @pending.settle()
  }
  guard @pending.leaf_live_count() == 0U else { panic() }
  guard @pending.leaf_drop_count() == expected_drops else { panic() }
}

///|
pub async fn run(background_group : @async-core.TaskGroup[Unit]) -> Unit {
  guard @pending.leaf_live_count() == 0U else { panic() }
  guard @pending.leaf_drop_count() == 0U else { panic() }

  @pending.backpressure_set(true)
  let leaf = @pending.Leaf::leaf()
  let future = @async-core.Future::ready_with_cleanup(leaf, fn(
    value : @pending.Leaf,
  ) {
    value.drop()
  })
  let (lowered, lowered_sink) = @async-core.Stream::new(capacity=1)
  let task = background_group.spawn(async fn() -> Unit {
    let signal : FixedArray[Unit] = [()]
    guard lowered_sink.write_all(signal[:]) else { panic() }
    lowered_sink.close()
    defer @pending.backpressure_set(false)
    @pending.pending(future)
  })

  guard lowered.read(1) is Some(signal) && signal.length() == 1 else { panic() }
  guard !@pending.pending_started() else { panic() }
  task.cancel()
  @pending.settle()
  guard !@pending.pending_started() else { panic() }

  settle_leaf_counts(1U)

  @pending.backpressure_set(true)
  let stream_leaf = @pending.Leaf::leaf()
  let stream_producer_ran = Ref(false)
  let stream = @async-core.Stream::produce(
    async fn(sink) {
      stream_producer_ran.val = true
      let values : FixedArray[@pending.Leaf] = [stream_leaf]
      guard sink.write_all(values[:]) else { panic() }
      sink.close()
    },
    on_unstarted_drop=fn() { stream_leaf.drop() },
  )
  let (stream_lowered, stream_lowered_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let stream_task = background_group.spawn(async fn() -> Unit {
    let signal : FixedArray[Unit] = [()]
    guard stream_lowered_sink.write_all(signal[:]) else { panic() }
    stream_lowered_sink.close()
    defer @pending.backpressure_set(false)
    @pending.pending_stream(stream)
  })

  guard stream_lowered.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  guard !@pending.pending_stream_started() else { panic() }
  stream_task.cancel()
  @pending.settle()
  guard !@pending.pending_stream_started() else { panic() }
  guard !stream_producer_ran.val else { panic() }

  settle_leaf_counts(2U)

  let returned_leaf = @pending.Leaf::leaf()
  let (return_started, return_started_sink) = @async-core.Stream::new(
    capacity=0,
  )
  let returned_task = background_group.spawn(async fn() -> Unit {
    let signal : FixedArray[Unit] = [()]
    guard return_started_sink.write_all(signal[:]) else { panic() }
    return_started_sink.close()
    let payload = @pending.return_after_cancel(returned_leaf)
    payload.leaf.drop()
  })
  guard return_started.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  guard @gate-control.started() else { panic() }
  returned_task.cancel()
  @gate-control.release()

  settle_leaf_counts(5U)

  let cleanup_leaf = fn(value : @pending.Leaf) { value.drop() }
  let stream_values : FixedArray[@async-core.Future[@pending.Leaf]] = [
    @async-core.Future::ready_with_cleanup(@pending.Leaf::leaf(), cleanup_leaf),
    @async-core.Future::ready_with_cleanup(@pending.Leaf::leaf(), cleanup_leaf),
    @async-core.Future::ready_with_cleanup(@pending.Leaf::leaf(), cleanup_leaf),
  ]
  let cleanup_stream = @async-core.Stream::produce(async fn(sink) {
    guard !sink.write_all(stream_values[:]) else {
      abort("peer unexpectedly accepted the future stream")
    }
    sink.close()
  })
  @pending.drop_future_stream(cleanup_stream)
  settle_leaf_counts(8U)

  let after_start_future = @async-core.Future::ready_with_cleanup(
    @pending.Leaf::leaf(),
    cleanup_leaf,
  )
  let after_start_stream_leaf = @pending.Leaf::leaf()
  let after_start_stream = @async-core.Stream::produce(
    async fn(sink) {
      let values : FixedArray[@pending.Leaf] = [after_start_stream_leaf]
      guard !sink.write_all(values[:]) else {
        abort("cancelled callee unexpectedly consumed its stream")
      }
      sink.close()
    },
    on_unstarted_drop=fn() { after_start_stream_leaf.drop() },
  )
  let after_start_task = background_group.spawn(async fn() -> Unit {
    @pending.take_after_start({
      direct: @pending.Leaf::leaf(),
      future_value: after_start_future,
      stream_value: after_start_stream,
    })
  })
  while !@pending.take_after_start_started() {
    @pending.settle()
  }
  after_start_task.cancel()
  after_start_task.wait() catch {
    _ => ()
  }
  settle_leaf_counts(11U)

  let race_future = @pending.open_race_future()
  let (race_read_started, race_read_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let race_read = background_group.spawn(async fn() -> Bool {
    let signal : FixedArray[Unit] = [()]
    guard race_read_started_sink.write_all(signal[:]) else { panic() }
    race_read_started_sink.close()
    let value = race_future.get()
    value.drop()
    true
  })
  guard race_read_started.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  @pending.complete_race_future()
  race_read.cancel()
  guard race_read.wait() else { panic() }
  settle_leaf_counts(12U)

  let race_stream = @pending.open_race_stream()
  let (race_stream_started, race_stream_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let race_stream_read = background_group.spawn(async fn() -> Bool {
    let signal : FixedArray[Unit] = [()]
    guard race_stream_started_sink.write_all(signal[:]) else { panic() }
    race_stream_started_sink.close()
    guard race_stream.read(1) is Some(values) && values.length() == 1 else {
      return false
    }
    values[0].drop()
    true
  })
  guard race_stream_started.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  @pending.complete_race_stream()
  race_stream_read.cancel()
  guard race_stream_read.wait() else { panic() }
  settle_leaf_counts(13U)

  let pending_future = @pending.open_future()
  let (future_read_started, future_read_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let future_read = background_group.spawn(async fn() -> Unit {
    let signal : FixedArray[Unit] = [()]
    guard future_read_started_sink.write_all(signal[:]) else { panic() }
    future_read_started_sink.close()
    let _ = pending_future.get() catch { _ => return }
  })
  guard future_read_started.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  pending_future.drop()
  future_read.wait()
  guard @pending.open_future_reader_dropped() else { panic() }

  let cancelled_future = @pending.open_future()
  let (future_cancel_started, future_cancel_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let future_cancel = background_group.spawn(
    async fn() -> Unit {
      let signal : FixedArray[Unit] = [()]
      guard future_cancel_started_sink.write_all(signal[:]) else { panic() }
      future_cancel_started_sink.close()
      let _ = cancelled_future.get()
    },
    allow_failure=true,
  )
  guard future_cancel_started.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  future_cancel.cancel()
  let future_was_cancelled = try future_cancel.wait() catch {
    @async-core.Cancelled::Cancelled => true
    _ => false
  } noraise {
    _ => false
  }
  guard future_was_cancelled else { panic() }
  guard @pending.open_future_reader_dropped() else { panic() }

  let pending_stream = @pending.open_stream()
  let (stream_read_started, stream_read_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let stream_read = background_group.spawn(async fn() -> Unit {
    let signal : FixedArray[Unit] = [()]
    guard stream_read_started_sink.write_all(signal[:]) else { panic() }
    stream_read_started_sink.close()
    let _ = pending_stream.read(1) catch { _ => return }
  })
  guard stream_read_started.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  pending_stream.drop()
  stream_read.wait()
  guard @pending.open_stream_reader_dropped() else { panic() }

  let cancelled_stream = @pending.open_stream()
  let (stream_cancel_started, stream_cancel_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let stream_cancel = background_group.spawn(
    async fn() -> Unit {
      let signal : FixedArray[Unit] = [()]
      guard stream_cancel_started_sink.write_all(signal[:]) else { panic() }
      stream_cancel_started_sink.close()
      let _ = cancelled_stream.read(1)
    },
    allow_failure=true,
  )
  guard stream_cancel_started.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  stream_cancel.cancel()
  let stream_was_cancelled = try stream_cancel.wait() catch {
    @async-core.Cancelled::Cancelled => true
    _ => false
  } noraise {
    _ => false
  }
  guard stream_was_cancelled else { panic() }
  guard @pending.open_stream_reader_dropped() else { panic() }
  guard @pending.leaf_live_count() == 0U else { panic() }
  guard @pending.leaf_drop_count() == 17U else { panic() }
}
