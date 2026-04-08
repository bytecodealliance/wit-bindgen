//@ [lang]
//@ path = 'gen/interface/my/test_/i/stub.mbt'

///|
let thing_values : Map[Int, UInt] = {}

///|
let next_thing_rep : Ref[Int] = { val: 1 }

///|
pub fn Thing::dtor(self : Thing) -> Unit {
  thing_values.remove(self.rep())
}

///|
pub fn Thing::thing(value : UInt) -> Thing {
  let rep = next_thing_rep.val
  next_thing_rep.val += 1
  thing_values[rep] = value
  Thing::new(rep)
}

///|
pub fn Thing::value(self : Thing) -> UInt {
  thing_values[self.rep()]
}

///|
pub fn active_things() -> UInt {
  thing_values.length().reinterpret_as_uint()
}

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

///|
pub async fn create_bridge_thing_stream_with_signal(
  signal : @async.Future[Unit],
  task_group : @async.TaskGroup[Unit],
) -> (@async.Stream[Thing], @async.Future[Bool]) {
  let (stream, sink) = @async.Stream::new(capacity=1)
  let (done, promise) = @async.Future::new()
  task_group.spawn_bg(async fn() {
    let first : Array[Thing] = [Thing::thing(0U)]
    let rest : Array[Thing] = [Thing::thing(1U), Thing::thing(2U), Thing::thing(3U)]
    assert_eq(sink.write(first[:]), 1)
    signal.get()
    let mut offset = 0
    for ;; {
      if offset >= rest.length() {
        promise.write(true)
        break
      }
      let written = sink.write(rest[offset:])
      if written <= 0 {
        for i = offset; i < rest.length(); i = i + 1 {
          rest[i].drop()
        }
        promise.write(false)
        break
      }
      offset += written
    }
    sink.close()
  })
  (stream, done)
}
