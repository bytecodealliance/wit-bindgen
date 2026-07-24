//@ wasmtime-flags = '-Wcomponent-model-async'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = """{ "supported-targets": "+wasm", "import": [{ "path": "test/moonbit-local-async-primitives/async-core", "alias": "async-core" }, { "path": "test/moonbit-local-async-primitives/interface/test/moonbit-local-async-primitives/operations", "alias": "operations" }] }"""

///|
pub async fn run(_background_group : @async-core.TaskGroup[Unit]) -> Unit {
  // Repeated exports exercise component-task state removal and recreation.
  for _ in 0..<3 {
    guard @operations.exercise() else { panic() }
  }
}
