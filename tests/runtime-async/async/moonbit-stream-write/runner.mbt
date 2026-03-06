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
}
