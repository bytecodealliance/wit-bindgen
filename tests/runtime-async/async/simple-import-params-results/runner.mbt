//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "import": ["a/b/interface/a/b/i", "a/b/async"] }"""

///|
pub async fn run(task_group : @async.TaskGroup[Unit]) -> Unit {
  @i.one_argument(1)
  assert_eq(@i.one_result(), 2)
  assert_eq(@i.one_argument_and_result(3), 4)
  @i.two_arguments(5, 6)
  assert_eq(@i.two_arguments_and_result(7, 8), 9)
}
