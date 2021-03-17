use anyhow::Result;
use wasmtime::*;

mod exports;
mod imports;

const CHECKED: &[u8] = include_bytes!(env!("CHECKED"));
const UNCHECKED: &[u8] = include_bytes!(env!("UNCHECKED"));

fn main() -> Result<()> {
    // Create an engine with caching enabled to assist with iteration in this
    // project.
    let mut config = Config::new();
    config.cache_config_load_default()?;
    let engine = Engine::new(&config)?;

    run_test(&engine, CHECKED)?;
    run_test(&engine, UNCHECKED)?;

    Ok(())
}

fn run_test(engine: &Engine, wasm: &[u8]) -> Result<()> {
    // Compile our wasm module ...
    let module = Module::new(&engine, wasm)?;

    // Create a linker with WASI functions ...
    let store = Store::new(&engine);
    let mut linker = Linker::new(&store);
    wasmtime_wasi::Wasi::new(
        &store,
        wasi_cap_std_sync::WasiCtxBuilder::new()
            .inherit_stdio()
            .build()?,
    )
    .add_to_linker(&mut linker)?;

    // Add our witx-defined functions to the linker
    imports::add_host_to_linker(imports::MyHost::default(), &mut linker)?;

    let exports = exports::Wasm::new(&module, &mut linker)?;
    exports::test(&exports)?;
    Ok(())
}
