use anyhow::Result;
use wasmer::{ImportObject, Instance, Module, Store};
use wasmer_wasi::WasiState;

test_helpers::runtime_tests_wasmer!();

fn instantiate<T>(
    wasm: &str,
    add_imports: impl FnOnce(&Store, &mut ImportObject),
    mk_exports: impl FnOnce(&Store, &Module, &mut ImportObject) -> Result<(T, Instance)>,
) -> Result<T> {
    let store = Store::default();
    let module = Module::from_file(&store, wasm)?;

    let mut wasi_env = WasiState::new("test").finalize()?;
    let mut import_object = wasi_env
        .import_object(&module)
        .unwrap_or(ImportObject::new());
    add_imports(&store, &mut import_object);

    let (exports, _instance) = mk_exports(&store, &module, &mut import_object)?;
    Ok(exports)
}
