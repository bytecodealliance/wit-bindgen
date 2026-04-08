//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": ["my/test/interface/my/test_/i", "my/test/async"] }"""

///|
pub async fn run(task_group : @async.TaskGroup[Unit]) -> Unit {
  let stream = @i.create_stream_with_values(3U)
  let mut total = 0U
  let mut count = 0U
  while stream.read(16) is Some(chunk) {
    for value in chunk {
      total += value
      count += 1U
    }
  }
  assert_eq(count, 3U)
  assert_eq(total, 3U)

  let unit_stream = @i.create_unit_stream(5U)
  let mut unit_count = 0U
  while unit_stream.read(16) is Some(chunk) {
    unit_count += chunk.length().reinterpret_as_uint()
  }
  assert_eq(unit_count, 5U)

  let (signal, promise) = @async.Future::new()
  let (bridge_stream, bridge_done) = @i.create_bridge_stream_with_signal(signal)
  match bridge_stream.read(1) {
    Some(chunk) => {
      assert_eq(chunk.length(), 1)
      assert_eq(chunk[0], 0U)
    }
    None => panic()
  }
  bridge_stream.close()
  promise.write(())
  assert_eq(bridge_done.get(), false)

  let (unit_signal, unit_promise) = @async.Future::new()
  let (bridge_unit_stream, bridge_unit_done) = @i.create_bridge_unit_stream_with_signal(unit_signal)
  match bridge_unit_stream.read(1) {
    Some(chunk) => assert_eq(chunk.length(), 1)
    None => panic()
  }
  bridge_unit_stream.close()
  unit_promise.write(())
  assert_eq(bridge_unit_done.get(), false)

  let (thing_signal, thing_promise) = @async.Future::new()
  let (thing_stream, thing_done) = @i.create_bridge_thing_stream_with_signal(thing_signal)
  let first_thing = match thing_stream.read(1) {
    Some(chunk) => {
      assert_eq(chunk.length(), 1)
      let thing = chunk[0]
      assert_eq(thing.value(), 0U)
      thing
    }
    None => panic()
  }
  thing_stream.close()
  thing_promise.write(())
  assert_eq(thing_done.get(), false)
  first_thing.drop()
  @async.pause()
  assert_eq(@i.active_things(), 0U)
}
