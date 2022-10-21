use anyhow::Result;

wit_bindgen_host_wasmtime_rust::generate!({
    import: "../../tests/runtime/smoke/imports.wit",
    default: "../../tests/runtime/smoke/exports.wit",
    name: "exports",
    async: true,
});

#[derive(Default)]
pub struct MyImports {
    hit: bool,
}

#[wit_bindgen_host_wasmtime_rust::async_trait]
impl imports::Imports for MyImports {
    async fn thunk(&mut self) -> Result<()> {
        self.hit = true;
        println!("in the host");
        Ok(())
    }
}

async fn run_async(wasm: &str) -> Result<()> {
    let (exports, mut store) = crate::instantiate_async(
        wasm,
        |linker| {
            imports::add_to_linker(
                linker,
                |cx: &mut crate::Context<MyImports>| -> &mut MyImports { &mut cx.imports },
            )
        },
        |store, module, linker| {
            Box::pin(async { Exports::instantiate_async(store, module, linker).await })
        },
    )
    .await?;

    exports.thunk(&mut store).await?;

    assert!(store.data().imports.hit);

    Ok(())
}
