//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": ["my/test/interface/my/test_/i", "my/test/async"] }"""

///|
pub async fn run(task_group : @async.TaskGroup[Unit]) -> Unit {
  @i.cancel_before_read(@async.Future::ready(1U))
  @i.start_read_then_cancel(@async.Future::ready(4U), @async.Future::ready(()))
}
