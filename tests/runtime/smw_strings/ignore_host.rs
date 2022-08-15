use anyhow::Context;

wit_bindgen_host_wasmtime_rust::export!("../../tests/runtime/smw_strings/imports.wit");

#[derive(Default)]
pub struct Host {
    pub f1_s: String,
    pub f2_called: bool,
    // pub f3_a: String,
    // pub f3_b: String,
    // pub f3_c: String,
}

impl imports::Imports for Host {
    fn f1(&mut self, s: &str) {
        self.f1_s = s.to_string();
    }

    fn f2(&mut self) -> String {
        self.f2_called = true;
        "36 chambers".into()
    }

    // fn f3(&mut self, a: &str, b: &str, c: &str) -> (String, String, String) {
    //     self.f3_a = a.into();
    //     self.f3_b = b.into();
    //     self.f3_c = c.into();
    //     (a.into(), b.into(), c.into())
    // }
}

wit_bindgen_host_wasmtime_rust::import!("../../tests/runtime/smw_strings/exports.wit");

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

    assert_eq!(store.data().imports.f1_s, "Hello, WIT!");

    assert!(store.data().imports.f2_called, "JS should have called `f2`");

    // assert_eq!(store.data().imports.f3_a, "");
    // assert_eq!(store.data().imports.f3_b, "🚀");
    // assert_eq!(store.data().imports.f3_c, "hello");

    // Test that the export instance behaves as we expect it to.

    exports
        .f1(&mut store, "Hello, WIT!")
        .context("calling the `f1` export should succeed")?;

    let s = exports
        .f2(&mut store)
        .context("calling the `f2` export should succeed")?;
    assert_eq!(s, "36 chambers");

    // let (a, b, c) = exports
    //     .f3(&mut store, "", "🚀", "hello")
    //     .context("calling the `f3` export should succeed")?;
    // assert_eq!(a, "");
    // assert_eq!(b, "🚀");
    // assert_eq!(c, "hello");

    Ok(())
}
