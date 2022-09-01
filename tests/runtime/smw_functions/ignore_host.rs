use anyhow::Context;

wit_bindgen_host_wasmtime_rust::export!("../../tests/runtime/smw_functions/imports.wit");

#[derive(Default)]
pub struct Host {
    pub f1_called: bool,
    pub f2_arg: u32,
    pub f3_a: u32,
    pub f3_b: u32,
    pub f4_called: bool,
    // pub f5_called: bool,
    // pub f6_a: u32,
    // pub f6_b: u32,
    // pub f6_c: u32,
}

impl imports::Imports for Host {
    fn f1(&mut self) {
        self.f1_called = true;
    }

    fn f2(&mut self, arg: u32) {
        self.f2_arg = arg;
    }

    fn f3(&mut self, a: u32, b: u32) {
        self.f3_a = a;
        self.f3_b = b;
    }

    fn f4(&mut self) -> u32 {
        self.f4_called = true;
        1337
    }

    // fn f5(&mut self) -> (u32, u32) {
    //     self.f5_called = true;
    //     (1, 2)
    // }

    // fn f6(&mut self, a: u32, b: u32, c: u32) -> (u32, u32, u32) {
    //     self.f6_a = a;
    //     self.f6_b = b;
    //     self.f6_c = c;
    //     (a + 1, b + 1, c + 1)
    // }
}

wit_bindgen_host_wasmtime_rust::import!("../../tests/runtime/smw_functions/exports.wit");

fn run(wasm: &str) -> anyhow::Result<()> {
    let (exports, mut store) = crate::instantiate_smw(
        wasm,
        |linker| imports::add_to_linker(linker, |cx| -> &mut Host { &mut cx.imports }),
        |store, module, linker| {
            exports::Exports::instantiate(store, module, linker, |cx| &mut cx.exports)
        },
    )?;

    // Test that the import instance called the functions we made available with
    // the expected arguments.

    exports.test_imports(&mut store)?;

    assert!(
        store.data().imports.f1_called,
        "top-level JS imported and called `f1`",
    );

    assert_eq!(
        store.data().imports.f2_arg,
        42,
        "f2 should have been called with 42",
    );

    assert_eq!(store.data().imports.f3_a, 0);
    assert_eq!(store.data().imports.f3_b, u32::MAX);

    assert!(
        store.data().imports.f4_called,
        "the top-level JS imported and called `f4`",
    );

    // assert!(
    //     store.data().imports.f5_called,
    //     "the top-level JS imported and called `f5`"
    // );

    // assert_eq!(store.data().imports.f6_a, 100);
    // assert_eq!(store.data().imports.f6_b, 200);
    // assert_eq!(store.data().imports.f6_c, 300);

    // Test that the export instance behaves as we expect it to.

    exports
        .f1(&mut store)
        .context("calling the `f1` export should succeed")?;

    exports
        .f2(&mut store, 42)
        .context("calling the `f2` export should succeed")?;

    exports
        .f3(&mut store, 0, u32::MAX)
        .context("calling the `f3` export should succeed")?;

    let a = exports
        .f4(&mut store)
        .context("calling the `f4` export should succeed")?;
    assert_eq!(a, 1337);

    // let (a, b) = exports
    //     .f5(&mut store)
    //     .context("calling the `f5` export should succeed")?;
    // assert_eq!(a, 1);
    // assert_eq!(b, 2);

    // let (a, b, c) = exports
    //     .f6(&mut store, 100, 200, 300)
    //     .context("calling the `f6` export should succeed")?;
    // assert_eq!(a, 101);
    // assert_eq!(b, 201);
    // assert_eq!(c, 301);

    Ok(())
}
