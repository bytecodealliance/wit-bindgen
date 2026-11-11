//@ wasmtime-flags = '-Wcomponent-model-async -S p3'
//@ [lang]
//@ path = 'gen/world/runner/stub.mbt'
//@ pkg_config = '''
//@ import {
//@   "my/test/async-core" @async-core,
//@   "my/test/interface/my/test/i" @i,
//@ }
//@
//@ warnings = "-44"
//@
//@ supported_targets = "wasm"
//@ '''

///|
pub async fn run(_background_group : @async-core.TaskGroup[Unit]) -> Unit {
  guard @i.write_stdout() is Ok(_) else { panic() }
}
