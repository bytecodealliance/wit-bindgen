use anyhow::Result;
use wasmtime::*;

mod exports;
mod imports;

#[allow(dead_code, type_alias_bounds)]
#[path = "../../../tmp/wasmtime/bindings.rs"]
mod wat;

const CHECKED: &[u8] = include_bytes!(env!("CHECKED"));
const UNCHECKED: &[u8] = include_bytes!(env!("UNCHECKED"));

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
}

fn run_test(engine: &Engine, wasm: &[u8]) -> Result<()> {
    // Compile our wasm module ...
    let module = Module::new(&engine, wasm)?;
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
        },
    );
    // let instance = linker.instantiate(&mut store, &module)?;

    let exports = exports::Wasm::new(&mut store, &module, &mut linker)?;
    exports::test(&exports, &mut store)?;
    Ok(())
}
