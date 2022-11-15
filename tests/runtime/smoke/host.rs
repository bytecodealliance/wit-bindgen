use anyhow::Result;

wit_bindgen_host_wasmtime_rust::generate!("../../tests/runtime/smoke/world.wit");

#[derive(Default)]
pub struct MyImports {
    hit: bool,
}

impl imports::Imports for MyImports {
    fn thunk(&mut self) -> Result<()> {
        self.hit = true;
        println!("in the host");
        Ok(())
    }
}

fn run(wasm: &str) -> Result<()> {
    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| {
            imports::add_to_linker(
                linker,
                |cx: &mut crate::Context<MyImports>| -> &mut MyImports { &mut cx.imports },
            )
        },
        |store, module, linker| Smoke::instantiate(store, module, linker),
    )?;

    exports.thunk(&mut store)?;

    assert!(store.data().imports.hit);

    Ok(())
}
