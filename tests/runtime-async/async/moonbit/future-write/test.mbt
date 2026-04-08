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
pub async fn create_future_with_value(value : UInt, task_group : @async.TaskGroup[Unit]) -> @async.Future[UInt] {
  @async.Future::ready(value)
}

///|
pub async fn create_unit_future(task_group : @async.TaskGroup[Unit]) -> @async.Future[Unit] {
  @async.Future::ready(())
}

///|
pub async fn create_nested_future(value : UInt, task_group : @async.TaskGroup[Unit]) -> @async.Future[@async.Future[UInt]] {
  let inner = @async.Future::ready(value)
  @async.Future::ready(inner)
}

///|
pub async fn create_nested_future_record(value : UInt, task_group : @async.TaskGroup[Unit]) -> NestedFutureRecord {
  let inner = @async.Future::ready(value)
  { nested: @async.Future::ready(inner) }
}

///|
pub async fn create_dropped_resource_future_with_signal(
  signal : @async.Future[Unit],
  task_group : @async.TaskGroup[Unit],
) -> (@async.Future[Thing], @async.Future[Unit]) {
  let (future, promise) = @async.Future::new()
  let (done, done_promise) = @async.Future::new()
  task_group.spawn_bg(async fn() {
    signal.get()
    promise.write(Thing::thing(7U))
    @async.pause()
    done_promise.write(())
  })
  (future, done)
}
