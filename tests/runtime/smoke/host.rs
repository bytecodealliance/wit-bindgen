use anyhow::Result;

witx_bindgen_wasmtime::import!("./tests/runtime/smoke/imports.witx");

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

witx_bindgen_wasmtime::export!("./tests/runtime/smoke/exports.witx");

fn run(wasm: &str) -> Result<()> {
    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| imports::add_imports_to_linker(linker, |cx| -> &mut MyImports { &mut cx.imports }),
        |store, module, linker| {
            exports::Exports::instantiate(store, module, linker, |cx| &mut cx.exports)
        },
    )?;

    exports.thunk(&mut store)?;

    assert!(store.data().imports.hit);

    Ok(())
}
