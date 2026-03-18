//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": ["my/test/interface/my/test_/i", "my/test/async"] }"""

///|
pub async fn run(task_group : @async.TaskGroup[Unit]) -> Unit {
  let stream = @async.Stream::from(async fn(sink : @async.Sink[Byte]) {
    let a : Array[Byte] = [(0).to_byte()]
    let b : Array[Byte] = [(1).to_byte(), (2).to_byte()]
    let c : Array[Byte] = [(3).to_byte()]
    let d : Array[Byte] = [(4).to_byte()]
    assert_eq(sink.write(a[:]), 1)
    assert_eq(sink.write(b[:]), 2)
    assert_eq(sink.write(c[:]), 1)
    assert_eq(sink.write(d[:]), 1)
    sink.close()
  })
  @i.read_stream(stream)
}
