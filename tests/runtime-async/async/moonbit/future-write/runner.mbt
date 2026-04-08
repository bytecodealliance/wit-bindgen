//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": ["my/test/interface/my/test_/i", "my/test/async"] }"""

///|
pub async fn run(task_group : @async.TaskGroup[Unit]) -> Unit {
  let rx = @i.create_future_with_value(42U)
  assert_eq(rx.get(), 42U)

  let done = @i.create_unit_future()
  done.get()

  let nested = @i.create_nested_future(7U)
  let inner = nested.get()
  assert_eq(inner.get(), 7U)

  let nested_record = @i.create_nested_future_record(9U)
  let nested_inner = nested_record.nested.get()
  assert_eq(nested_inner.get(), 9U)

  let (signal, promise) = @async.Future::new()
  let (resource_future, done) = @i.create_dropped_resource_future_with_signal(signal)
  resource_future.drop()
  promise.write(())
  done.get()
  @async.pause()
  assert_eq(@i.active_things(), 0U)
}
