use anyhow::Result;
use wasmtime::{Config, Engine, Linker, Module, Store};
use witx_bindgen_wasmtime::HostFuture;

witx_bindgen_wasmtime::export!({
    async: *,
    paths: ["./tests/runtime/async_raw/imports.witx"],
});

#[derive(Default)]
struct MyImports;

impl imports::Imports for MyImports {
    fn thunk(&mut self) -> HostFuture<()> {
        Box::pin(async {})
    }
}

witx_bindgen_wasmtime::import!({
    async: *,
    paths: ["./tests/runtime/async_raw/exports.witx"],
});

fn run(wasm: &str) -> Result<()> {
    struct Context {
        wasi: wasmtime_wasi::WasiCtx,
        imports: MyImports,
        exports: exports::ExportsData,
    }

    let mut config = Config::new();
    config.async_support(true);
    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, wasm)?;
    let mut linker = Linker::<Context>::new(&engine);
    imports::add_to_linker(&mut linker, |cx| &mut cx.imports)?;
    wasmtime_wasi::add_to_linker(&mut linker, |cx| &mut cx.wasi)?;
    exports::Exports::add_to_linker(&mut linker, |cx| &mut cx.exports)?;

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let instantiate = || async {
            let mut store = Store::new(
                &engine,
                Context {
                    wasi: crate::default_wasi(),
                    imports: MyImports::default(),
                    exports: Default::default(),
                },
            );
            let instance = linker.instantiate_async(&mut store, &module).await?;
            exports::Exports::new(store, &instance, |cx| &mut cx.exports)
        };

        // it's ok to call the completion callback immediately, and the first
        // coroutine is always zero.
        let exports = instantiate().await?;
        exports.complete_immediately().await?;
        exports.assert_coroutine_id_zero().await?;
        exports.assert_coroutine_id_zero().await?;

        // if the completion callback is never called that's a trap
        let err = instantiate()
            .await?
            .completion_not_called()
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("completion callback never called"),
            "bad error: {}",
            err,
        );

        // the completion callback can only be called once
        let err = instantiate().await?.complete_twice().await.unwrap_err();
        assert!(
            err.to_string().contains("async context not valid"),
            "bad error: {}",
            err,
        );

        // if the trap happens after the completion callback... something
        // happens, for now a trap.
        let err = instantiate().await?.complete_then_trap().await.unwrap_err();
        assert!(
            err.trap_code() == Some(wasmtime::TrapCode::UnreachableCodeReached),
            "bad error: {:?}",
            err
        );

        // If a non-async export tries to call the completion callback for
        // async exports that's an error.
        let err = instantiate()
            .await?
            .not_async_export_done()
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("async context not valid"),
            "bad error: {}",
            err,
        );

        // If a non-async export tries to call an async import that's an error.
        let err = instantiate()
            .await?
            .not_async_calls_async()
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("cannot call async import from non-async export"),
            "bad error: {}",
            err,
        );

        // The import callback specified cannot be null
        let err = instantiate()
            .await?
            .import_callback_null()
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("callback was a null function"),
            "bad error: {}",
            err,
        );

        // The import callback specified must have the right type.
        let err = instantiate()
            .await?
            .import_callback_wrong_type()
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("type mismatch with parameters"),
            "bad error: {}",
            err,
        );

        // The import callback specified must point to a valid table index
        let err = instantiate()
            .await?
            .import_callback_bad_index()
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("invalid function index"),
            "bad error: {}",
            err,
        );

        // when wasm traps due to one reason or another all future requests to
        // execute wasm fail
        let exports = instantiate().await?;
        assert!(exports.import_callback_null().await.is_err());
        assert!(exports.complete_immediately().await.is_err());

        Ok(())
    })
}
