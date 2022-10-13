#![allow(type_alias_bounds)] // TODO: should fix generated code to not fire this

use anyhow::Result;
use wasmtime::{
    component::{Component, Instance, Linker},
    Config, Engine, Store,
};
use wit_bindgen_testwasi_host_wasmtime_rust as testwasi;

test_helpers::runtime_tests_wasmtime!();

fn default_config() -> Result<Config> {
    // Create an engine with caching enabled to assist with iteration in this
    // project.
    let mut config = Config::new();
    config.cache_config_load_default()?;
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.wasm_component_model(true);
    Ok(config)
}

struct Context<I> {
    imports: I,
    testwasi: testwasi::TestWasi,
}

fn instantiate<I: Default, T>(
    wasm: &str,
    add_imports: impl FnOnce(&mut Linker<Context<I>>) -> Result<()>,
    mk_exports: impl FnOnce(
        &mut Store<Context<I>>,
        &Component,
        &mut Linker<Context<I>>,
    ) -> Result<(T, Instance)>,
) -> Result<(T, Store<Context<I>>)> {
    let engine = Engine::new(&default_config()?)?;
    let module = Component::from_file(&engine, wasm)?;

    let mut linker = Linker::new(&engine);
    add_imports(&mut linker)?;
    testwasi::add_to_linker(&mut linker, |cx| &mut cx.testwasi)?;

    let mut store = Store::new(
        &engine,
        Context {
            imports: I::default(),
            testwasi: testwasi::TestWasi::default(),
        },
    );
    let (exports, _instance) = mk_exports(&mut store, &module, &mut linker)?;
    Ok((exports, store))
}
