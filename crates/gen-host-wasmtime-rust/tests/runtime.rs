use anyhow::Result;
use wasmtime::{
    component::{Component, Instance, Linker},
    Config, Engine, Store,
};

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

#[derive(Default)]
struct Context<I> {
    imports: I,
    testwasi: TestWasi,
}

fn instantiate<I: Default, T>(
    wasm: &str,
    add_imports: impl FnOnce(&mut Linker<Context<I>>) -> Result<()>,
    mk_exports: impl FnOnce(
        &mut Store<Context<I>>,
        &Component,
        &Linker<Context<I>>,
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
            testwasi: TestWasi::default(),
        },
    );
    let (exports, _instance) = mk_exports(&mut store, &module, &linker)?;
    Ok((exports, store))
}

async fn instantiate_async<F, I: Default, T: Send>(
    wasm: &str,
    add_imports: impl FnOnce(&mut Linker<Context<I>>) -> Result<()>,
    mk_exports: F,
) -> Result<(T, Store<Context<I>>)>
where
    F: for<'a> FnOnce(
        &'a mut Store<Context<I>>,
        &'a Component,
        &'a Linker<Context<I>>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<(T, Instance)>> + 'a>,
    >,
{
    let mut config = default_config()?;
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let module = Component::from_file(&engine, wasm)?;

    let mut linker = Linker::new(&engine);
    add_imports(&mut linker)?;
    testwasi::add_to_linker(&mut linker, |cx| &mut cx.testwasi)?;

    let mut store = Store::new(
        &engine,
        Context {
            imports: I::default(),
            testwasi: TestWasi::default(),
        },
    );
    let (exports, _instance) = mk_exports(&mut store, &module, &linker).await?;
    Ok((exports, store))
}

wit_bindgen_host_wasmtime_rust::generate!("../wasi_snapshot_preview1/testwasi.wit");

#[derive(Default)]
pub struct TestWasi;

impl testwasi::Testwasi for TestWasi {
    fn log(&mut self, bytes: Vec<u8>) -> Result<()> {
        match std::str::from_utf8(&bytes) {
            Ok(s) => print!("{}", s),
            Err(_) => println!("\nbinary: {:?}", bytes),
        }
        Ok(())
    }

    fn log_err(&mut self, bytes: Vec<u8>) -> Result<()> {
        match std::str::from_utf8(&bytes) {
            Ok(s) => eprint!("{}", s),
            Err(_) => eprintln!("\nbinary: {:?}", bytes),
        }
        Ok(())
    }
}
