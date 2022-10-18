use anyhow::Result;

wit_bindgen_host_wasmtime_rust::generate!({
    import: "../../tests/runtime/smoke/imports.wit",
    default: "../../tests/runtime/smoke/exports.wit",
    name: "exports",
});

#[derive(Default)]
pub struct MyImports {
    hit: bool,
}

impl imports::Imports for MyImports {
    fn thunk(&mut self) {
        self.hit = true;
        println!("in the host");
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
        |store, module, linker| Exports::instantiate(store, module, linker),
    )?;

    exports.thunk(&mut store)?;

    assert!(store.data().imports.hit);

    Ok(())
}
