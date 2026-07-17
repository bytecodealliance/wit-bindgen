//@ [lang]
//@ path = 'gen/interface/test/moonbit-stream-write-cancel/controller/stub.mbt'
//@ pkg_config = """{ "import": [{ "path": "test/moonbit-stream-write-cancel/async-core", "alias": "async-core" }, { "path": "test/moonbit-stream-write-cancel/interface/test/moonbit-stream-write-cancel/holder", "alias": "holder" }] }"""

///|
let never : (@async-core.Future[Unit], @async-core.Promise[Unit]) =
  @async-core.Future::new()

///|
let cancellation_seen : Ref[Bool] = { val: false }

///|
let cancellation_done : (@async-core.Future[Unit], @async-core.Promise[Unit]) =
  @async-core.Future::new()

///|
let second_write_seen : Ref[Bool] = { val: false }

///|
let peer_drop_producer_finished : Ref[Bool] = { val: false }

///|
let producer_done : (@async-core.Future[Unit], @async-core.Promise[Unit]) =
  @async-core.Future::new()

///|
pub async fn run_until_cancelled(
  _background_group : @async-core.TaskGroup[Unit]
) -> Unit {
  let stream = @async-core.Stream::produce(async fn(sink) {
    defer {
      cancellation_seen.val = true
      ignore(cancellation_done.1.complete(()))
    }
    @holder.mark_write_started()
    let data : FixedArray[@holder.Leaf] = [@holder.Leaf::leaf()]
    let _ = sink.write_all(data[:])
  })
  @holder.hold(stream)
  never.0.get()
}

///|
pub fn cancellation_observed() -> Bool {
  cancellation_seen.val
}

///|
pub async fn wait_cancellation_observed(
  _background_group : @async-core.TaskGroup[Unit]
) -> Unit {
  cancellation_done.0.get()
}

///|
pub async fn start_peer_drop(
  _background_group : @async-core.TaskGroup[Unit]
) -> Unit {
  let stream = @async-core.Stream::produce(async fn(sink) {
    defer {
      peer_drop_producer_finished.val = true
      ignore(producer_done.1.complete(()))
    }
    let first : FixedArray[@holder.Leaf] = [@holder.Leaf::leaf()]
    guard sink.write_all(first[:]) else { panic() }
    let second : FixedArray[@holder.Leaf] = [@holder.Leaf::leaf()]
    guard sink.write_all(second[:]) else { panic() }
    second_write_seen.val = true
    let third : FixedArray[@holder.Leaf] = [@holder.Leaf::leaf()]
    guard !sink.write_all(third[:]) else { panic() }
    let fourth : FixedArray[@holder.Leaf] = [@holder.Leaf::leaf()]
    guard !sink.write_all(fourth[:]) else { panic() }
  }, cleanup=fn(value) { value.drop() })
  guard stream.read(1) is Some(priming) && priming.length() == 1 else {
    panic()
  }
  priming[0].drop()
  @holder.hold(stream)
}

///|
pub fn second_write_started() -> Bool {
  second_write_seen.val
}

///|
pub fn producer_finished() -> Bool {
  peer_drop_producer_finished.val
}

///|
pub async fn wait_producer_finished(
  _background_group : @async-core.TaskGroup[Unit]
) -> Unit {
  producer_done.0.get()
}
