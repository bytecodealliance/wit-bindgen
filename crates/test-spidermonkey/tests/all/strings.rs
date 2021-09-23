use anyhow::Context;

pub mod imports {
    witx_bindgen_wasmtime::import!("crates/test-spidermonkey/tests/strings.witx");
    pub use strings::add_strings_to_linker;

    #[derive(Default)]
    pub struct Host {
        pub f1_s: String,
        pub f2_called: bool,
        pub f3_a: String,
        pub f3_b: String,
        pub f3_c: String,
    }

    impl strings::Strings for Host {
        fn f1(&mut self, s: &str) {
            self.f1_s = s.to_string();
        }

        fn f2(&mut self) -> String {
            self.f2_called = true;
            "36 chambers".into()
        }

        fn f3(&mut self, a: &str, b: &str, c: &str) -> (String, String, String) {
            self.f3_a = a.into();
            self.f3_b = b.into();
            self.f3_c = c.into();
            (a.into(), b.into(), c.into())
        }
    }
}

pub mod exports {
    witx_bindgen_wasmtime::export!("crates/test-spidermonkey/tests/strings.witx");
}

#[test]
fn test() -> anyhow::Result<()> {
    let (mut store, _export_linker, _export_module, export_instance) = test_spidermonkey::run_test(
        "./tests/strings.witx",
        (
            imports::Host::default(),
            wasmtime_wasi::sync::WasiCtxBuilder::new()
                .inherit_stdio()
                .build(),
        ),
        |engine| {
            let mut linker = wasmtime::Linker::new(engine);
            wasmtime_wasi::add_to_linker(&mut linker, |(_, wasi)| wasi)?;
            imports::add_strings_to_linker(&mut linker, |(host, _)| host)?;
            Ok(linker)
        },
    )?;

    // Test that the import instance called the functions we made available with
    // the expected arguments.

    assert_eq!(store.data().0.f1_s, "Hello, WITX!");

    assert!(store.data().0.f2_called, "JS should have called `f2`");

    assert_eq!(store.data().0.f3_a, "");
    assert_eq!(store.data().0.f3_b, "🚀");
    assert_eq!(store.data().0.f3_c, "hello");

    // Test that the export instance behaves as we expect it to.

    let export_instance = exports::strings::Strings::new(&mut store, &export_instance, |_| todo!())
        .context("should create `Strings` object from instance")?;

    export_instance
        .f1(&mut store, "Hello, WITX!")
        .context("calling the `f1` export should succeed")?;

    let s = export_instance
        .f2(&mut store)
        .context("calling the `f2` export should succeed")?;
    assert_eq!(s, "36 chambers");

    let (a, b, c) = export_instance
        .f3(&mut store, "", "🚀", "hello")
        .context("calling the `f3` export should succeed")?;
    assert_eq!(a, "");
    assert_eq!(b, "🚀");
    assert_eq!(c, "hello");

    Ok(())
}
