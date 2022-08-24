use anyhow::{Context as _, Result};
use wasmtime::{Config, Engine, Instance, Linker, Module, Store};

test_helpers::runtime_tests_wasmtime!();

fn default_config() -> Result<Config> {
    // Create an engine with caching enabled to assist with iteration in this
    // project.
    let mut config = Config::new();
    config.cache_config_load_default()?;
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    Ok(config)
}

fn default_wasi() -> wasmtime_wasi::WasiCtx {
    wasmtime_wasi::sync::WasiCtxBuilder::new()
        .inherit_stdio()
        .build()
}

struct Context<I, E> {
    wasi: wasmtime_wasi::WasiCtx,
    imports: I,
    exports: E,
}

fn instantiate<I: Default, E: Default, T>(
    wasm: &str,
    add_imports: impl FnOnce(&mut Linker<Context<I, E>>) -> Result<()>,
    mk_exports: impl FnOnce(
        &mut Store<Context<I, E>>,
        &Module,
        &mut Linker<Context<I, E>>,
    ) -> Result<(T, Instance)>,
) -> Result<(T, Store<Context<I, E>>)> {
    let engine = Engine::new(&default_config()?)?;
    let module = Module::from_file(&engine, wasm)?;

    let mut linker = Linker::new(&engine);
    add_imports(&mut linker)?;
    wasmtime_wasi::add_to_linker(&mut linker, |cx| &mut cx.wasi)?;

    let mut store = Store::new(
        &engine,
        Context {
            wasi: default_wasi(),
            imports: I::default(),
            exports: E::default(),
        },
    );
    let (exports, _instance) = mk_exports(&mut store, &module, &mut linker)?;
    Ok((exports, store))
}

// TODO: This function needs to be updated to use the component model once it's ready.  See
// https://github.com/bytecodealliance/wit-bindgen/issues/259 for details.
//
// Also, rename the ignore_host.rs files under the tests/runtime/smw_{functions|lists|strings} to host.rs and
// remove the leading underscore from this function's name to re-enable the Spidermonkey tests.
fn _instantiate_smw<I: Default, E: Default, T>(
    wasm: &str,
    add_imports: impl FnOnce(&mut Linker<Context<I, E>>) -> Result<()>,
    mk_exports: impl FnOnce(
        &mut Store<Context<I, E>>,
        &Module,
        &mut Linker<Context<I, E>>,
    ) -> Result<(T, Instance)>,
) -> Result<(T, Store<Context<I, E>>)> {
    let mut config = default_config()?;
    config.wasm_multi_memory(true);
    let engine = Engine::new(&config)?;

    println!("reading wasms...");
    let wasm = std::fs::read(wasm).context(format!("failed to read {}", wasm))?;
    let smw = std::fs::read("../gen-host-spidermonkey-js/spidermonkey-wasm/spidermonkey.wasm")
        .context("failed to read `spidermonkey.wasm`")?;
    println!("compiling input wasm...");
    let module = Module::new(&engine, &wasm)?;
    println!("compiling spidermonkey.wasm...");
    let smw = Module::new(&engine, &smw)?;

    let mut linker = Linker::new(&engine);
    add_imports(&mut linker)?;
    wasmtime_wasi::add_to_linker(&mut linker, |cx| &mut cx.wasi)?;

    let mut store = Store::new(
        &engine,
        Context {
            wasi: default_wasi(),
            imports: I::default(),
            exports: E::default(),
        },
    );

    println!("instantiating spidermonkey.wasm...");
    let _smw_instance = linker
        .instantiate(&mut store, &smw)
        .context("failed to instantiate `spidermonkey.wasm`")?;
    // TODO: replace this with a component model equivalent:
    // linker.define_name("spidermonkey", smw_instance)?;

    println!("instantiating input wasm...");
    let (exports, instance) = mk_exports(&mut store, &module, &mut linker)?;

    println!("running wizer.initialize");
    let init = instance.get_typed_func::<(), (), _>(&mut store, "wizer.initialize")?;
    init.call(&mut store, ())
        .context("failed to call wizer.initialize")?;
    Ok((exports, store))
}
