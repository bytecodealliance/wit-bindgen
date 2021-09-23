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

fn instantiate_smw<I: Default, E: Default, T>(
    wasm: &str,
    add_imports: impl FnOnce(&mut Linker<Context<I, E>>) -> Result<()>,
    mk_exports: impl FnOnce(
        &mut Store<Context<I, E>>,
        &Module,
        &mut Linker<Context<I, E>>,
    ) -> Result<(T, Instance)>,
) -> Result<(T, Store<Context<I, E>>)> {
    let mut config = default_config()?;
    config.wasm_module_linking(true);
    config.wasm_multi_memory(true);
    let engine = Engine::new(&config)?;

    println!("reading wasms...");
    let wasm = std::fs::read(wasm).context(format!("failed to read {}", wasm))?;
    let smw = std::fs::read("../gen-spidermonkey/spidermonkey-wasm/spidermonkey.wasm")
        .context("failed to read `spidermonkey.wasm`")?;
    println!("compiling input wasm...");
    let module = Module::new_with_name(&engine, &wasm, "wasm.wasm")?;
    println!("compiling spidermonkey.wasm...");
    let smw = Module::new_with_name(&engine, &smw, "spidermonkey.wasm")?;

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
    let smw_instance = linker
        .instantiate(&mut store, &smw)
        .context("failed to instantiate `spidermonkey.wasm`")?;
    linker.define_name("spidermonkey", smw_instance)?;

    println!("instantiating input wasm...");
    let (exports, instance) = mk_exports(&mut store, &module, &mut linker)?;

    println!("running wizer.initialize");
    let init = instance.get_typed_func::<(), (), _>(&mut store, "wizer.initialize")?;
    init.call(&mut store, ())
        .context("failed to call wizer.initialize")?;
    Ok((exports, store))
}
