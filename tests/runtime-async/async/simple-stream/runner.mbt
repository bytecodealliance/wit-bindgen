//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": ["my/test/interface/my/test_/i", "my/test/async"] }"""

///|
pub async fn run(task_group : @async.TaskGroup[Unit]) -> Unit {
  let stream = @async.CMStream::from(async fn(sink : @async.Sink[Unit]) {
    let first : Array[Unit] = [()]
    let second : Array[Unit] = [(), ()]
    assert_eq(sink.write(first[:]), 1)
    assert_eq(sink.write(second[:]), 2)
    sink.close()
  })
  @i.read_stream(stream)
}
