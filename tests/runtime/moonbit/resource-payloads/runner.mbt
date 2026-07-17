//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": [{ "path": "my/test/async-core", "alias": "async-core" }, { "path": "my/test/interface/my/test/leaf-interface", "alias": "leaf-interface" }, { "path": "my/test/interface/my/test/test-interface", "alias": "test-interface" }] }"""

///|
async fn read_one_leaf(
  stream : @async-core.Stream[@leaf-interface.LeafThing],
  expected : String,
) -> Unit {
  match stream.read(1) {
    Some(things) => {
      if things.length() != 1 {
        for thing in things {
          thing.drop()
        }
        panic()
      }
      let thing = things[0]
      let value = thing.get()
      thing.drop()
      guard value == expected else { panic() }
    }
    None => panic()
  }
}

///|
async fn wait_for_leaf_drop_count(expected : UInt) -> Unit {
  for _ in 0..<32 {
    if @leaf-interface.leaf_drop_count() == expected {
      return
    }
    @test-interface.settle()
  }
  guard @leaf-interface.leaf_drop_count() == expected else { panic() }
}

///|
async fn assert_no_live_resources() -> Unit {
  for _ in 0..<32 {
    if @leaf-interface.leaf_live_count() == 0U &&
      @leaf-interface.fields_live_count() == 0U &&
      @leaf-interface.body_live_count() == 0U &&
      @leaf-interface.response_live_count() == 0U {
      return
    }
    @test-interface.settle()
  }
  guard @leaf-interface.leaf_live_count() == 0U else { panic() }
  guard @leaf-interface.fields_live_count() == 0U else { panic() }
  guard @leaf-interface.body_live_count() == 0U else { panic() }
  guard @leaf-interface.response_live_count() == 0U else { panic() }
}

///|
fn nested_leaf_stream(
  prefix : String,
) -> @async-core.Stream[@leaf-interface.LeafThing] {
  @async-core.Stream::produce(async fn(sink) {
    let values : FixedArray[@leaf-interface.LeafThing] = [
      @leaf-interface.LeafThing::leaf_thing(prefix + "-a"),
      @leaf-interface.LeafThing::leaf_thing(prefix + "-b"),
    ]
    let _ = sink.write_all(values[:])
    sink.close()
  })
}

///|
fn nested_leaf_future(
  prefix : String,
) -> @async-core.Future[
  @async-core.Future[@async-core.Stream[@leaf-interface.LeafThing]],
] {
  @async-core.Future::ready(
    @async-core.Future::ready(nested_leaf_stream(prefix)),
  )
}

///|
pub async fn run(background_group : @async-core.TaskGroup[Unit]) -> Unit {
  {
    let before = @leaf-interface.leaf_drop_count()
    let output = @test-interface.relay_nested_leaf(
      nested_leaf_future("nested-transfer"),
    )
    let stream = output.get().get()
    read_one_leaf(stream, "nested-transfer-a")
    read_one_leaf(stream, "nested-transfer-b")
    guard stream.read(1) is None else { panic() }
    stream.drop()
    output.drop()
    wait_for_leaf_drop_count(before + 2U)
  }
  {
    let before = @leaf-interface.leaf_drop_count()
    let output = @test-interface.relay_nested_leaf(
      nested_leaf_future("nested-reject"),
    )
    output.drop()
    wait_for_leaf_drop_count(before + 2U)
  }
  {
    let before = @leaf-interface.leaf_drop_count()
    let input = @async-core.Stream::produce(async fn(sink) {
      let values : FixedArray[@async-core.Future[@leaf-interface.LeafThing]] = [
        @async-core.Future::ready(
          @leaf-interface.LeafThing::leaf_thing("future-stream-a"),
        ),
        @async-core.Future::ready(
          @leaf-interface.LeafThing::leaf_thing("future-stream-b"),
        ),
      ]
      let _ = sink.write_all(values[:])
      sink.close()
    })
    let output = @test-interface.relay_leaf_futures(input)
    guard output.read(1) is Some(values) && values.length() == 1 else {
      panic()
    }
    let leaf = values[0].get()
    guard leaf.get() == "future-stream-a" else { panic() }
    leaf.drop()
    output.drop()
    wait_for_leaf_drop_count(before + 2U)
  }
  {
    let (input, sink) = @async-core.Stream::new_with_cleanup(
      fn(value : @leaf-interface.LeafThing) { value.drop() },
      capacity=3,
    )
    let values : FixedArray[@leaf-interface.LeafThing] = [
      @leaf-interface.LeafThing::leaf_thing("a"),
      @leaf-interface.LeafThing::leaf_thing("b"),
      @leaf-interface.LeafThing::leaf_thing("c"),
    ]
    guard sink.write_all(values[:]) else { panic() }
    sink.close()

    let stream = @test-interface.short_reads_leaf(input)
    read_one_leaf(stream, "a")
    read_one_leaf(stream, "b")
    read_one_leaf(stream, "c")
    stream.drop()
  }
  {
    let before = @leaf-interface.leaf_drop_count()
    let (input, sink) = @async-core.Stream::new_with_cleanup(
      fn(value : @leaf-interface.LeafThing) { value.drop() },
      capacity=3,
    )
    let values : FixedArray[@leaf-interface.LeafThing] = [
      @leaf-interface.LeafThing::leaf_thing("drop-a"),
      @leaf-interface.LeafThing::leaf_thing("drop-b"),
      @leaf-interface.LeafThing::leaf_thing("drop-c"),
    ]
    guard sink.write_all(values[:]) else { panic() }
    sink.close()

    let stream = @test-interface.short_reads_leaf(input)
    read_one_leaf(stream, "drop-a")
    stream.drop()
    wait_for_leaf_drop_count(before + 3U)
  }
  {
    let (release_first, release_first_sink) = @async-core.Stream::new(
      capacity=1,
    )
    let first_payload = @async-core.Future::ready_with_cleanup(
      @leaf-interface.LeafThing::leaf_thing("a"),
      fn(value : @leaf-interface.LeafThing) { value.drop() },
    )
    let first = @async-core.Future::from(async fn() {
      guard release_first.read(1) is Some(signal) && signal.length() == 1 else {
        abort("future release signal closed early")
      }
      first_payload.get()
    })
    let second = @async-core.Future::ready_with_cleanup(
      @leaf-interface.LeafThing::leaf_thing("a"),
      fn(value : @leaf-interface.LeafThing) { value.drop() },
    )
    let (out1, out2) = @test-interface.dropped_reader_leaf(first, second)

    out1.drop()
    let thing = out2.get()
    let value = thing.get()
    thing.drop()
    out2.drop()
    guard value == "a" else { panic() }

    // `out2` is only produced after the peer has dropped `first`.
    let signal : FixedArray[Unit] = [()]
    guard release_first_sink.write_all(signal[:]) else { panic() }
    release_first_sink.close()
    assert_no_live_resources()
  }
  {
    let before = @leaf-interface.leaf_drop_count()
    let (release, release_sink) = @async-core.Stream::new(capacity=1)
    let payload = @async-core.Future::ready_with_cleanup(
      @leaf-interface.LeafThing::leaf_thing("cancelled-future-read"),
      fn(value : @leaf-interface.LeafThing) { value.drop() },
    )
    let future = @async-core.Future::from(async fn() {
      guard release.read(1) is Some(signal) && signal.length() == 1 else {
        abort("future release signal closed early")
      }
      payload.get()
    })
    guard @test-interface.cancel_future_read(future) else { panic() }

    let signal : FixedArray[Unit] = [()]
    guard release_sink.write_all(signal[:]) else { panic() }
    release_sink.close()
    wait_for_leaf_drop_count(before + 1U)
  }
  {
    let before = @leaf-interface.leaf_drop_count()
    let (stream, sink) = @async-core.Stream::new_with_cleanup(
      fn(value : @leaf-interface.LeafThing) { value.drop() },
      capacity=0,
    )
    guard @test-interface.cancel_stream_read(stream) else { panic() }

    let values : FixedArray[@leaf-interface.LeafThing] = [
      @leaf-interface.LeafThing::leaf_thing("cancelled-stream-read"),
    ]
    let _ = sink.write_all(values[:])
    sink.close()
    wait_for_leaf_drop_count(before + 1U)
  }
  {
    let before = @leaf-interface.leaf_drop_count()
    let stream = @test-interface.make_leaf_stream()
    read_one_leaf(stream, "stream-a")
    stream.drop()
    wait_for_leaf_drop_count(before + 4U)
  }
  {
    let stream = @test-interface.make_local_leaf_stream()
    read_one_leaf(stream, "local-a")
    read_one_leaf(stream, "local-b")
    read_one_leaf(stream, "local-c")
    match stream.read(1) {
      None => ()
      Some(things) => {
        for thing in things {
          thing.drop()
        }
        panic()
      }
    }
    stream.drop()
  }
  {
    let response = @test-interface.make_response()
    let (body, trailers) = response.collect()
    response.drop()
    guard body == "hello http p3" else { panic() }
    guard trailers == Some("done") else { panic() }
  }
  {
    let response = @test-interface.make_response_post_work()
    let before = @test-interface.post_response_count()
    guard before < 3U else { panic() }

    let (body, trailers) = response.collect()
    response.drop()
    guard body == "post work" else { panic() }
    guard trailers == Some("after-body") else { panic() }
    guard @test-interface.post_response_count() >= 3U else { panic() }
  }
  {
    let response = @test-interface.make_response_background()
    guard @test-interface.background_count() == 0U else { panic() }

    let (body, trailers) = response.collect()
    response.drop()
    guard body == "background" else { panic() }
    guard trailers == Some("background-done") else { panic() }
    guard @test-interface.background_count() == 1U else { panic() }
  }
  {
    let future = @test-interface.delayed_leaf_future()
    let thing = future.get()
    let value = thing.get()
    thing.drop()
    future.drop()
    guard value == "delayed" else { panic() }
  }

  guard @test-interface.local_future_drop_cleanup() == 1U else { panic() }
  guard @test-interface.local_future_drop_resource_cleanup() == 1U else {
    panic()
  }
  guard @test-interface.local_lazy_future_drop_resource_cleanup() == 1U else {
    panic()
  }
  guard @test-interface.local_stream_drop_resource_cleanup() == 4U else {
    panic()
  }
  guard @test-interface.local_lazy_stream_drop_resource_cleanup() == 1U else {
    panic()
  }
  guard @test-interface.local_lazy_stream_partial_drop_resource_cleanup() == 2U else {
    panic()
  }
  guard @test-interface.local_stream_cancelled_write_resource_cleanup() == 3U else {
    panic()
  }
  guard @test-interface.local_stream_rendezvous() else { panic() }
  guard @test-interface.local_stream_bounded_backpressure() else { panic() }
  guard @test-interface.local_stream_cancelled_waiters() else { panic() }
  guard @test-interface.local_stream_produce_read() else { panic() }
  assert_no_live_resources()

  // The export ABI supplies this group even when this runner has no detached work.
  ignore(background_group)
}
