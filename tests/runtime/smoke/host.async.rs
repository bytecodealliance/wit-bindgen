use anyhow::Result;

wit_bindgen_host_wasmtime_rust::generate!({
    path: "../../tests/runtime/smoke/world.wit",
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
            Box::pin(async { Smoke::instantiate_async(store, module, linker).await })
        },
    )
    .await?;

    exports.thunk(&mut store).await?;

    assert!(store.data().imports.hit);

    Ok(())
}
