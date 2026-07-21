//@ wasmtime-flags = '-Wcomponent-model-async -S p3'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "warn-list": "-44", "supported-targets": "+wasm", "import": [{ "path": "my/test/async-core", "alias": "async-core" }, { "path": "my/test/interface/my/test/i", "alias": "i" }] }"""

///|
pub async fn run(_background_group : @async-core.TaskGroup[Unit]) -> Unit {
  guard @i.write_stdout() is Ok(_) else { panic() }
}
