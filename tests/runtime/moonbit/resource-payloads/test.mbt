//@ [lang]
//@ path = 'gen/interface/my/test/test-interface/stub.mbt'

///|
let post_response_steps : Ref[UInt] = Ref(0)

///|
fn record_post_response_step() -> Unit {
  post_response_steps.val += 1
}

///|
let background_steps : Ref[UInt] = Ref(0)

///|
fn reset_background_work() -> Unit {
  background_steps.val = 0
}

///|
pub async fn settle(_background_group : @async-core.TaskGroup[Unit]) -> Unit {

}

///|
pub async fn short_reads_leaf(
  s : @async-core.Stream[@leaf-interface.LeafThing],
  _background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Stream[@leaf-interface.LeafThing] {
  @async-core.Stream::produce(async fn(sink) {
    let things : Array[@leaf-interface.LeafThing] = []
    for ;; {
      match s.read(1) {
        Some(chunk) =>
          for i in 0..<chunk.length() {
            things.push(chunk[i])
          }
        None => break
      }
    }
    let data = FixedArray::from_array(things)
    let _ = sink.write_all(data[:])
    sink.close()
  })
}

///|
pub async fn dropped_reader_leaf(
  f1 : @async-core.Future[@leaf-interface.LeafThing],
  f2 : @async-core.Future[@leaf-interface.LeafThing],
  _background_group : @async-core.TaskGroup[Unit],
) -> (
  @async-core.Future[@leaf-interface.LeafThing],
  @async-core.Future[@leaf-interface.LeafThing],
) {
  let value : Ref[String?] = Ref(None)
  let (value_ready, value_ready_sink) = @async-core.Stream::new(capacity=1)
  let out1 = @async-core.Future::from(async fn() {
    f1.drop()
    let thing = f2.get()
    value.val = Some(thing.get())
    let signal : FixedArray[Unit] = [()]
    guard value_ready_sink.write_all(signal[:]) else {
      abort("future value signal closed early")
    }
    value_ready_sink.close()
    thing
  })
  let out2 = @async-core.Future::from(async fn() {
    guard value_ready.read(1) is Some(signal) && signal.length() == 1 else {
      abort("future value signal closed early")
    }
    @leaf-interface.LeafThing::leaf_thing(value.val.unwrap())
  })
  (out1, out2)
}

///|
pub async fn make_leaf_stream(
  _background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Stream[@leaf-interface.LeafThing] {
  @async-core.Stream::produce(async fn(sink) {
    let data : FixedArray[@leaf-interface.LeafThing] = [
      @leaf-interface.LeafThing::leaf_thing("stream-a"),
      @leaf-interface.LeafThing::leaf_thing("stream-b"),
      @leaf-interface.LeafThing::leaf_thing("stream-c"),
      @leaf-interface.LeafThing::leaf_thing("stream-d"),
    ]
    let _ = sink.write_all(data[:])
    sink.close()
  })
}

///|
pub async fn make_local_leaf_stream(
  background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Stream[@leaf-interface.LeafThing] {
  let (stream, sink) = @async-core.Stream::new_with_cleanup(
    fn(value : @leaf-interface.LeafThing) { value.drop() },
    capacity=1,
  )
  background_group.spawn_bg(async fn() {
    let data : FixedArray[@leaf-interface.LeafThing] = [
      @leaf-interface.LeafThing::leaf_thing("local-a"),
      @leaf-interface.LeafThing::leaf_thing("local-b"),
      @leaf-interface.LeafThing::leaf_thing("local-c"),
    ]
    let _ = sink.write_all(data[:])
    sink.close()
  })
  stream
}

///|
pub async fn make_response(
  _background_group : @async-core.TaskGroup[Unit],
) -> @leaf-interface.Response {
  let trailers = @leaf-interface.Fields::fields("done")
  let body = @leaf-interface.Body::body(
    @async-core.Stream::produce(async fn(sink) {
      let _ = sink.write_all_bytes(b"hello "[:])
      let _ = sink.write_all_bytes(b"http p3"[:])
      sink.close()
    }),
    Some(@async-core.Future::ready(trailers)),
  )
  @leaf-interface.Response::response(body)
}

///|
pub async fn make_response_post_work(
  _background_group : @async-core.TaskGroup[Unit],
) -> @leaf-interface.Response {
  let (body_done, body_done_sink) = @async-core.Stream::new(capacity=1)
  let trailers = @async-core.Future::from(async fn() {
    guard body_done.read(1) is Some(signal) && signal.length() == 1 else {
      abort("body completion signal closed early")
    }
    guard post_response_steps.val >= 3 else {
      abort("body completion signal arrived too early")
    }
    @leaf-interface.Fields::fields("after-body")
  })
  let body = @leaf-interface.Body::body(
    @async-core.Stream::produce(async fn(sink) {
      record_post_response_step()
      let _ = sink.write_all_bytes(b"post "[:])
      record_post_response_step()
      let _ = sink.write_all_bytes(b"work"[:])
      sink.close()
      record_post_response_step()
      let signal : FixedArray[Unit] = [()]
      guard body_done_sink.write_all(signal[:]) else {
        abort("body completion signal closed early")
      }
      body_done_sink.close()
    }),
    Some(trailers),
  )
  @leaf-interface.Response::response(body)
}

///|
pub fn post_response_count() -> UInt {
  post_response_steps.val
}

///|
pub async fn make_response_background(
  background_group : @async-core.TaskGroup[Unit],
) -> @leaf-interface.Response {
  reset_background_work()
  let (body_done, body_done_sink) = @async-core.Stream::new(capacity=1)
  let (background_done, background_done_sink) = @async-core.Stream::new(
    capacity=1,
  )
  background_group.spawn_bg(async fn() {
    guard body_done.read(1) is Some(signal) && signal.length() == 1 else {
      abort("body completion signal closed early")
    }
    background_steps.val += 1
    let signal : FixedArray[Unit] = [()]
    guard background_done_sink.write_all(signal[:]) else {
      abort("background completion signal closed early")
    }
    background_done_sink.close()
  })
  let trailers = @async-core.Future::from(async fn() {
    guard background_done.read(1) is Some(signal) && signal.length() == 1 else {
      abort("background completion signal closed early")
    }
    @leaf-interface.Fields::fields("background-done")
  })
  let body = @leaf-interface.Body::body(
    @async-core.Stream::produce(async fn(sink) {
      let _ = sink.write_all_bytes(b"background"[:])
      sink.close()
      let signal : FixedArray[Unit] = [()]
      guard body_done_sink.write_all(signal[:]) else {
        abort("body completion signal closed early")
      }
      body_done_sink.close()
    }),
    Some(trailers),
  )
  @leaf-interface.Response::response(body)
}

///|
pub fn background_count() -> UInt {
  background_steps.val
}

///|
pub async fn delayed_leaf_future(
  background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Future[@leaf-interface.LeafThing] {
  let (ready, ready_sink) = @async-core.Stream::new(capacity=0)
  background_group.spawn_bg(async fn() {
    let signal : FixedArray[Unit] = [()]
    guard ready_sink.write_all(signal[:]) else { return }
    ready_sink.close()
  })
  @async-core.Future::from(async fn() {
    guard ready.read(1) is Some(signal) && signal.length() == 1 else {
      abort("future readiness signal closed early")
    }
    @leaf-interface.LeafThing::leaf_thing("delayed")
  })
}

///|
pub async fn local_future_drop_cleanup(
  _background_group : @async-core.TaskGroup[Unit],
) -> UInt {
  let counts : Array[UInt] = [0]
  let f = @async-core.Future::ready_with_cleanup("owned", fn(_value) {
    counts[0] = counts[0] + 1
  })
  f.drop()
  let unstarted_cleanup_ran = Ref(false)
  let started = @async-core.Future::from(async fn() { 42 }, on_unstarted_drop=fn() {
    unstarted_cleanup_ran.val = true
  })
  guard started.get() == 42 else {
    abort("future producer returned wrong value")
  }
  started.drop()
  guard !unstarted_cleanup_ran.val else {
    abort("started future ran its unstarted cleanup")
  }
  counts[0]
}

///|
pub async fn local_future_drop_resource_cleanup(
  _background_group : @async-core.TaskGroup[Unit],
) -> UInt {
  let before = @leaf-interface.leaf_drop_count()
  let thing = @leaf-interface.LeafThing::leaf_thing("cleanup")
  let f = @async-core.Future::ready_with_cleanup(thing, fn(value) {
    value.drop()
  })
  f.drop()
  @leaf-interface.leaf_drop_count() - before
}

///|
pub async fn local_lazy_future_drop_resource_cleanup(
  _background_group : @async-core.TaskGroup[Unit],
) -> UInt {
  let before = @leaf-interface.leaf_drop_count()
  let producer_ran = Ref(false)
  let thing = @leaf-interface.LeafThing::leaf_thing("lazy-future-cleanup")
  let future = @async-core.Future::from(
    async fn() {
      producer_ran.val = true
      thing
    },
    on_unstarted_drop=fn() { thing.drop() },
  )
  future.drop()
  guard !producer_ran.val else {
    abort("dropping an unstarted future ran its producer")
  }
  @leaf-interface.leaf_drop_count() - before
}

///|
pub async fn local_stream_drop_resource_cleanup(
  _background_group : @async-core.TaskGroup[Unit],
) -> UInt {
  let before = @leaf-interface.leaf_drop_count()
  let (stream, sink) = @async-core.Stream::new_with_cleanup(
    fn(value : @leaf-interface.LeafThing) { value.drop() },
    capacity=4,
  )
  let data : FixedArray[@leaf-interface.LeafThing] = [
    @leaf-interface.LeafThing::leaf_thing("buffered-a"),
    @leaf-interface.LeafThing::leaf_thing("buffered-b"),
    @leaf-interface.LeafThing::leaf_thing("buffered-c"),
    @leaf-interface.LeafThing::leaf_thing("buffered-d"),
  ]
  let _ = sink.write_all(data[:])
  sink.close()
  match stream.read(1) {
    Some(chunk) => chunk[0].drop()
    None => panic()
  }
  stream.drop()
  @leaf-interface.leaf_drop_count() - before
}

///|
pub async fn local_lazy_stream_drop_resource_cleanup(
  _background_group : @async-core.TaskGroup[Unit],
) -> UInt {
  let before = @leaf-interface.leaf_drop_count()
  let producer_ran = Ref(false)
  let thing = @leaf-interface.LeafThing::leaf_thing("lazy-stream-cleanup")
  let stream : @async-core.Stream[@leaf-interface.LeafThing] = @async-core.Stream::produce(
    async fn(sink) {
      producer_ran.val = true
      let values : FixedArray[@leaf-interface.LeafThing] = [thing]
      guard sink.write_all(values[:]) else { return }
      sink.close()
    },
    on_unstarted_drop=fn() { thing.drop() },
  )
  stream.drop()
  guard !producer_ran.val else {
    abort("dropping an unstarted stream ran its producer")
  }
  @leaf-interface.leaf_drop_count() - before
}

///|
pub async fn local_lazy_stream_partial_drop_resource_cleanup(
  _background_group : @async-core.TaskGroup[Unit],
) -> UInt {
  let before = @leaf-interface.leaf_drop_count()
  let (producer_done, producer_done_promise) = @async-core.Future::new()
  let stream : @async-core.Stream[@leaf-interface.LeafThing] = @async-core.Stream::produce(
    async fn(sink) {
      let values : FixedArray[@leaf-interface.LeafThing] = [
        @leaf-interface.LeafThing::leaf_thing("partial-a"),
        @leaf-interface.LeafThing::leaf_thing("partial-b"),
      ]
      ignore(producer_done_promise.complete(sink.write_all(values[:])))
    },
    cleanup=fn(value) { value.drop() },
  )
  guard stream.read(1) is Some(values) && values.length() == 1 else { panic() }
  values[0].drop()
  stream.drop()
  guard !producer_done.get() else { panic() }
  @leaf-interface.leaf_drop_count() - before
}

///|
pub async fn local_stream_cancelled_write_resource_cleanup(
  background_group : @async-core.TaskGroup[Unit],
) -> UInt {
  let before = @leaf-interface.leaf_drop_count()
  let (stream, sink) = @async-core.Stream::new_with_cleanup(
    fn(value : @leaf-interface.LeafThing) { value.drop() },
    capacity=1,
  )
  let (write_blocked, write_blocked_sink) = @async-core.Stream::new(capacity=1)
  let writer = background_group.spawn(
    async fn() -> Unit {
      let data : FixedArray[@leaf-interface.LeafThing] = [
        @leaf-interface.LeafThing::leaf_thing("cancelled-a"),
        @leaf-interface.LeafThing::leaf_thing("cancelled-b"),
        @leaf-interface.LeafThing::leaf_thing("cancelled-c"),
      ]
      let accepted = sink.write(data[:])
      guard accepted == 1 else { panic() }
      let signal : FixedArray[Unit] = [()]
      guard write_blocked_sink.write_all(signal[:]) else { panic() }
      write_blocked_sink.close()
      let _ = sink.write_all(data[accepted:])
    },
    allow_failure=true,
  )
  guard write_blocked.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  writer.cancel()
  writer.wait() catch {
    _ => ()
  }
  guard @leaf-interface.leaf_drop_count() - before == 2 else { panic() }
  stream.drop()
  @leaf-interface.leaf_drop_count() - before
}

///|
pub async fn cancel_future_read(
  future : @async-core.Future[@leaf-interface.LeafThing],
  background_group : @async-core.TaskGroup[Unit],
) -> Bool {
  let (read_started, read_started_sink) = @async-core.Stream::new(capacity=1)
  let reader = background_group.spawn(
    async fn() -> Bool {
      let signal : FixedArray[Unit] = [()]
      guard read_started_sink.write_all(signal[:]) else { panic() }
      read_started_sink.close()
      let value = future.get()
      value.drop()
      false
    },
    allow_failure=true,
  )
  guard read_started.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  reader.cancel()
  try reader.wait() catch {
    @async-core.Cancelled::Cancelled => true
    _ => false
  } noraise {
    _ => false
  }
}

///|
pub async fn cancel_stream_read(
  stream : @async-core.Stream[@leaf-interface.LeafThing],
  background_group : @async-core.TaskGroup[Unit],
) -> Bool {
  let (read_started, read_started_sink) = @async-core.Stream::new(capacity=1)
  let reader = background_group.spawn(
    async fn() -> Bool {
      let signal : FixedArray[Unit] = [()]
      guard read_started_sink.write_all(signal[:]) else { panic() }
      read_started_sink.close()
      match stream.read(1) {
        Some(chunk) =>
          for value in chunk {
            value.drop()
          }
        None => ()
      }
      false
    },
    allow_failure=true,
  )
  guard read_started.read(1) is Some(signal) && signal.length() == 1 else {
    panic()
  }
  reader.cancel()
  try reader.wait() catch {
    @async-core.Cancelled::Cancelled => true
    _ => false
  } noraise {
    _ => false
  }
}

///|
pub async fn relay_nested_leaf(
  value : @async-core.Future[
    @async-core.Future[@async-core.Stream[@leaf-interface.LeafThing]],
  ],
  _background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Future[
  @async-core.Future[@async-core.Stream[@leaf-interface.LeafThing]],
] {
  @async-core.Future::from(async fn() {
    let inner = value.get()
    @async-core.Future::from(async fn() { inner.get() })
  })
}

///|
pub async fn relay_leaf_futures(
  value : @async-core.Stream[@async-core.Future[@leaf-interface.LeafThing]],
  _background_group : @async-core.TaskGroup[Unit],
) -> @async-core.Stream[@async-core.Future[@leaf-interface.LeafThing]] {
  value
}

///|
pub async fn local_stream_rendezvous(
  background_group : @async-core.TaskGroup[Unit],
) -> Bool {
  let (stream, sink) = @async-core.Stream::new(capacity=0)
  let phase = Ref(0)
  let (writer_started, writer_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let writer = background_group.spawn(async fn() -> Bool {
    let data : FixedArray[Int] = [42]
    phase.val = 1
    let signal : FixedArray[Unit] = [()]
    guard writer_started_sink.write_all(signal[:]) else { return false }
    writer_started_sink.close()
    guard sink.write_all(data[:]) else { return false }
    phase.val = 2
    sink.close()
    true
  })
  guard writer_started.read(1) is Some(signal) && signal.length() == 1 else {
    return false
  }
  guard phase.val == 1 else { return false }
  guard stream.read(1) is Some(chunk) && chunk.length() == 1 else {
    return false
  }
  writer.wait() && phase.val == 2 && stream.read(1) is None
}

///|
pub async fn local_stream_bounded_backpressure(
  background_group : @async-core.TaskGroup[Unit],
) -> Bool {
  let (stream, sink) = @async-core.Stream::new(capacity=2)
  let phase = Ref(0)
  let (write_blocked, write_blocked_sink) = @async-core.Stream::new(capacity=1)
  let writer = background_group.spawn(
    async fn() -> Bool {
      let data : FixedArray[Int] = [1, 2, 3]
      phase.val = 1
      let first = sink.write(data[:])
      guard first == 2 else { return false }
      phase.val = 2
      let signal : FixedArray[Unit] = [()]
      guard write_blocked_sink.write_all(signal[:]) else { return false }
      write_blocked_sink.close()
      let second = sink.write(data[first:])
      guard second == 1 else { return false }
      phase.val = 3
      sink.close()
      true
    },
    allow_failure=true,
  )
  guard write_blocked.read(1) is Some(signal) && signal.length() == 1 else {
    return false
  }
  guard phase.val == 2 else { return false }
  guard stream.read(1) is Some(first) else { return false }
  guard first.length() == 1 && first[0] == 1 else { return false }
  guard writer.wait() else { return false }
  guard phase.val == 3 else { return false }
  guard stream.read(2) is Some(second) else { return false }
  guard second.length() == 1 && second[0] == 2 else { return false }
  guard stream.read(2) is Some(third) else { return false }
  guard third.length() == 1 && third[0] == 3 else { return false }
  stream.read(1) is None
}

///|
pub async fn local_stream_cancelled_waiters(
  background_group : @async-core.TaskGroup[Unit],
) -> Bool {
  let pre_start_cleanup = Ref(false)
  let (never, never_sink) = @async-core.Stream::new(capacity=0)
  let pre_start = background_group.spawn(async fn() -> Unit {
    defer {
      pre_start_cleanup.val = true
    }
    let _ = never.read(1)
  })
  pre_start.cancel()
  pre_start.wait() catch {
    _ => ()
  }
  ignore(never_sink)
  guard pre_start_cleanup.val else { return false }

  let (stream, sink) = @async-core.Stream::new(capacity=0)

  let (writer_started, writer_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let blocked_writer = background_group.spawn(async fn() -> Int {
    let data : FixedArray[Int] = [1]
    let signal : FixedArray[Unit] = [()]
    guard writer_started_sink.write_all(signal[:]) else { return -1 }
    writer_started_sink.close()
    sink.write(data[:])
  })
  guard writer_started.read(1) is Some(signal) && signal.length() == 1 else {
    return false
  }
  blocked_writer.cancel()
  let blocked_writer_result = blocked_writer.wait() catch { _ => -1 }
  guard blocked_writer_result == -1 else { return false }

  let (reader_started, reader_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let blocked_reader = background_group.spawn(
    async fn() -> Int {
      let signal : FixedArray[Unit] = [()]
      guard reader_started_sink.write_all(signal[:]) else { return -1 }
      reader_started_sink.close()
      match stream.read(1) {
        Some(_) => 1
        None => 0
      }
    },
    allow_failure=true,
  )
  guard reader_started.read(1) is Some(signal) && signal.length() == 1 else {
    return false
  }
  blocked_reader.cancel()
  let blocked_reader_result = blocked_reader.wait() catch { _ => -1 }
  guard blocked_reader_result == -1 else { return false }

  let writer = background_group.spawn(
    async fn() -> Int {
      let data : FixedArray[Int] = [7]
      sink.write(data[:])
    },
    allow_failure=true,
  )
  guard stream.read(1) is Some(chunk) else { return false }
  guard chunk.length() == 1 && chunk[0] == 7 else { return false }
  writer.cancel()
  guard writer.wait() == 1 else { return false }

  let (finishing_reader_started, finishing_reader_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let finishing_reader = background_group.spawn(
    async fn() -> Int {
      let signal : FixedArray[Unit] = [()]
      guard finishing_reader_started_sink.write_all(signal[:]) else {
        return -1
      }
      finishing_reader_started_sink.close()
      match stream.read(1) {
        Some(data) if data.length() == 1 => data[0]
        _ => -1
      }
    },
    allow_failure=true,
  )
  guard finishing_reader_started.read(1) is Some(signal) && signal.length() == 1 else {
    return false
  }
  let data : FixedArray[Int] = [9]
  guard sink.write(data[:]) == 1 else { return false }
  finishing_reader.cancel()
  guard finishing_reader.wait() == 9 else { return false }

  sink.close()
  guard stream.read(1) is None else { return false }

  let (closed_stream, closed_sink) = @async-core.Stream::new(capacity=0)
  let (close_writer_started, close_writer_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let close_writer = background_group.spawn(async fn() -> Int {
    let signal : FixedArray[Unit] = [()]
    guard close_writer_started_sink.write_all(signal[:]) else { return -1 }
    close_writer_started_sink.close()
    let data : FixedArray[Int] = [11]
    closed_sink.write(data[:])
  })
  guard close_writer_started.read(1) is Some(signal) && signal.length() == 1 else {
    return false
  }
  closed_sink.close()
  close_writer.cancel()
  guard close_writer.wait() == 0 && !closed_sink.is_open() else { return false }
  guard closed_stream.read(1) is None else { return false }

  let (dropped_stream, dropped_sink) = @async-core.Stream::new(capacity=0)
  let (drop_writer_started, drop_writer_started_sink) = @async-core.Stream::new(
    capacity=1,
  )
  let drop_writer = background_group.spawn(async fn() -> Int {
    let signal : FixedArray[Unit] = [()]
    guard drop_writer_started_sink.write_all(signal[:]) else { return -1 }
    drop_writer_started_sink.close()
    let data : FixedArray[Int] = [13]
    dropped_sink.write(data[:])
  })
  guard drop_writer_started.read(1) is Some(signal) && signal.length() == 1 else {
    return false
  }
  dropped_stream.drop()
  drop_writer.cancel()
  drop_writer.wait() == 0 && !dropped_sink.is_open()
}

///|
pub async fn local_stream_produce_read(
  _background_group : @async-core.TaskGroup[Unit],
) -> Bool {
  let unstarted_cleanup_ran = Ref(false)
  let stream = @async-core.Stream::produce(
    async fn(sink) {
      let data : FixedArray[Int] = [1, 2, 3]
      let _ = sink.write_all(data[:])
      sink.close()
    },
    on_unstarted_drop=fn() { unstarted_cleanup_ran.val = true },
  )
  guard stream.read(2) is Some(first) else { return false }
  guard first.length() == 2 && first[0] == 1 && first[1] == 2 else {
    return false
  }
  guard stream.read(2) is Some(second) else { return false }
  guard second.length() == 1 && second[0] == 3 else { return false }
  guard stream.read(1) is None else { return false }
  stream.drop()
  !unstarted_cleanup_ran.val
}
