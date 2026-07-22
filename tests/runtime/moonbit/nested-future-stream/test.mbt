//@ [lang]
//@ path = 'gen/interface/test/moonbit-nested-future-stream/nested/stub.mbt'

///|
let shared : (@async-core.Future[UInt], @async-core.Promise[UInt]) = @async-core.Future::new()

///|
let never : (@async-core.Future[Unit], @async-core.Promise[Unit]) = @async-core.Future::new()

///|
let cancellation_seen : Ref[Bool] = { val: false }

///|
let cancellation_done : (@async-core.Future[Unit], @async-core.Promise[Unit]) = @async-core.Future::new()

///|
pub async fn relay(
  value : @async-core.Future[@async-core.Future[@async-core.Stream[Byte]]],
  _background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Future[@async-core.Future[@async-core.Stream[Byte]]] {
  value
}

///|
pub async fn relay_stream(
  value : @async-core.Stream[@async-core.Future[Byte]],
  _background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Stream[@async-core.Future[Byte]] {
  value
}

///|
pub async fn concurrent_writes(
  background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Stream[Byte] {
  let (first_sinks, first_sinks_sink) : (
    @async-core.Stream[@async-core.Sink[Byte]],
    @async-core.Sink[@async-core.Sink[Byte]],
  ) = @async-core.Stream::new(capacity=1)
  let (second_sinks, second_sinks_sink) : (
    @async-core.Stream[@async-core.Sink[Byte]],
    @async-core.Sink[@async-core.Sink[Byte]],
  ) = @async-core.Stream::new(capacity=1)
  let (first_started, first_started_sink) = @async-core.Stream::new(capacity=0)
  let first = background_group.spawn(
    async fn() -> Bool {
      guard first_sinks.read(1) is Some(sinks) && sinks.length() == 1 else {
        return false
      }
      let signal : FixedArray[Unit] = [()]
      guard first_started_sink.write_all(signal[:]) else { return false }
      first_started_sink.close()
      let data : FixedArray[Byte] = [1]
      sinks[0].write_all(data[:])
    },
    allow_failure=true,
  )
  let second = background_group.spawn(
    async fn() -> Bool {
      guard second_sinks.read(1) is Some(sinks) && sinks.length() == 1 else {
        return false
      }
      guard first_started.read(1) is Some(signal) && signal.length() == 1 else {
        return false
      }
      let data : FixedArray[Byte] = [2]
      sinks[0].write_all(data[:])
    },
    allow_failure=true,
  )
  @async-core.Stream::produce(async fn(sink) {
    let sinks : FixedArray[@async-core.Sink[Byte]] = [sink]
    guard first_sinks_sink.write_all(sinks[:]) else {
      abort("failed to start first component stream writer")
    }
    first_sinks_sink.close()
    guard second_sinks_sink.write_all(sinks[:]) else {
      abort("failed to start second component stream writer")
    }
    second_sinks_sink.close()
    guard first.wait() else { abort("first component stream write failed") }
    guard second.wait() else { abort("second component stream write failed") }
    sink.close()
  })
}

///|
pub async fn post_return_lazy(
  _background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Stream[Byte] {
  @async-core.Stream::produce(async fn(sink) {
    let nested = @async-core.Stream::produce(async fn(nested_sink) {
      let data : FixedArray[Byte] = [42]
      guard nested_sink.write_all(data[:]) else {
        abort("nested stream closed before accepting its value")
      }
      nested_sink.close()
    })
    guard nested.read(1) is Some(data) && data.length() == 1 else {
      abort("nested stream did not produce its value")
    }
    guard sink.write_all(data[:]) else {
      abort("component stream closed before accepting its value")
    }
    nested.drop()
    sink.close()
  })
}

///|
pub async fn wait_shared(
  _background_group : @async-core.TaskGroup[Unit],
) -> UInt {
  shared.0.get()
}

///|
pub async fn resolve_shared(
  value : UInt,
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  guard shared.1.complete(value) else { panic() }
}

///|
pub async fn wait_cancelled(
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  defer {
    cancellation_seen.val = true
    ignore(cancellation_done.1.complete(()))
  }
  never.0.get()
}

///|
pub async fn wait_cancellation_observed(
  _background_group : @async-core.TaskGroup[Unit],
) -> Unit {
  cancellation_done.0.get()
}

///|
pub fn cancellation_observed() -> Bool {
  cancellation_seen.val
}
