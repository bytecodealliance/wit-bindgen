use anyhow::Result;
use test_build_rust_wasm::*;
use wasmtime::*;

mod exports;
mod imports;

fn main() -> Result<()> {
    // Create an engine with caching enabled to assist with iteration in this
    // project.
    let mut config = Config::new();
    config.cache_config_load_default()?;
    let engine = Engine::new(&config)?;

    println!("Using CHECKED...");
    run_test(&engine, CHECKED)?;
    println!("Using UNCHECKED...");
    run_test(&engine, UNCHECKED)?;
    println!("Success!");

    Ok(())
}

pub struct Context {
    wasi: wasmtime_wasi::WasiCtx,
    imports: imports::MyHost,
    tables: imports::HostTables<imports::MyHost>,
    export_data: exports::WasmData,
}

fn run_test(engine: &Engine, wasm: &str) -> Result<()> {
    // Compile our wasm module ...
    let module = Module::from_file(&engine, wasm)?;
    let mut linker = Linker::<Context>::new(&engine);

    // Add WASI/witx functions to the linker
    wasmtime_wasi::add_to_linker(&mut linker, |cx| &mut cx.wasi)?;
    imports::add_host_to_linker(&mut linker, |cx| (&mut cx.imports, &mut cx.tables))?;

    // Create a linker with WASI functions ...
    let mut store = Store::new(
        &engine,
        Context {
            wasi: wasmtime_wasi::sync::WasiCtxBuilder::new()
                .inherit_stdio()
                .build(),
            imports: Default::default(),
            tables: Default::default(),
            export_data: Default::default(),
        },
    );

    let (exports, instance) =
        exports::Wasm::instantiate(&mut store, &module, &mut linker, |state| {
            &mut state.export_data
        })?;
    exports::test(&exports, instance, &mut store)?;
    Ok(())
}
