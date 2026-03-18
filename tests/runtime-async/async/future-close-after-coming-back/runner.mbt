//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": ["a/b/interface/a/b/theTest", "a/b/async"] }"""

///|
pub async fn run(task_group : @async.TaskGroup[Unit]) -> Unit {
  let returned = @theTest.f(@async.Future::ready(()))
  returned.drop()
}
