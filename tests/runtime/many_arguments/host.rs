use anyhow::Result;

wit_bindgen_wasmtime::export!("../../tests/runtime/many_arguments/imports.wit");

#[derive(Default)]
pub struct MyImports {}

impl imports::Imports for MyImports {
    fn many_arguments(
        &mut self,
        a1: u64,
        a2: u64,
        a3: u64,
        a4: u64,
        a5: u64,
        a6: u64,
        a7: u64,
        a8: u64,
        a9: u64,
        a10: u64,
        a11: u64,
        a12: u64,
        a13: u64,
        a14: u64,
        a15: u64,
        a16: u64,
        a17: u64,
        a18: u64,
        a19: u64,
        a20: u64,
    ) {
        assert_eq!(a1, 1);
        assert_eq!(a2, 2);
        assert_eq!(a3, 3);
        assert_eq!(a4, 4);
        assert_eq!(a5, 5);
        assert_eq!(a6, 6);
        assert_eq!(a7, 7);
        assert_eq!(a8, 8);
        assert_eq!(a9, 9);
        assert_eq!(a10, 10);
        assert_eq!(a11, 11);
        assert_eq!(a12, 12);
        assert_eq!(a13, 13);
        assert_eq!(a14, 14);
        assert_eq!(a15, 15);
        assert_eq!(a16, 16);
        assert_eq!(a17, 17);
        assert_eq!(a18, 18);
        assert_eq!(a19, 19);
        assert_eq!(a20, 20);
    }
}

wit_bindgen_wasmtime::import!("../../tests/runtime/many_arguments/exports.wit");

fn run(wasm: &str) -> Result<()> {
    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| imports::add_to_linker(linker, |cx| -> &mut MyImports { &mut cx.imports }),
        |store, module, linker| {
            exports::Exports::instantiate(store, module, linker, |cx| &mut cx.exports)
        },
    )?;

    exports.many_arguments(
        &mut store, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    )?;

    Ok(())
}
