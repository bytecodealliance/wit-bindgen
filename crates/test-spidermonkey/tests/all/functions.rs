use anyhow::Context;

pub mod imports {
    witx_bindgen_wasmtime::import!("crates/test-spidermonkey/tests/functions.witx");
    pub use functions::add_functions_to_linker;

    #[derive(Default)]
    pub struct Host {
        pub f1_called: bool,
        pub f2_arg: u32,
        pub f3_a: u32,
        pub f3_b: u32,
        pub f4_called: bool,
        pub f5_called: bool,
        pub f6_a: u32,
        pub f6_b: u32,
        pub f6_c: u32,
    }

    impl functions::Functions for Host {
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

        fn f5(&mut self) -> (u32, u32) {
            self.f5_called = true;
            (1, 2)
        }

        fn f6(&mut self, a: u32, b: u32, c: u32) -> (u32, u32, u32) {
            self.f6_a = a;
            self.f6_b = b;
            self.f6_c = c;
            (a + 1, b + 1, c + 1)
        }
    }
}

pub mod exports {
    witx_bindgen_wasmtime::export!("crates/test-spidermonkey/tests/functions.witx");
}

#[test]
fn test() -> anyhow::Result<()> {
    let (mut store, _export_linker, _export_module, export_instance) = test_spidermonkey::run_test(
        "./tests/functions.witx",
        (
            imports::Host::default(),
            wasmtime_wasi::sync::WasiCtxBuilder::new()
                .inherit_stdio()
                .build(),
        ),
        |engine| {
            let mut linker = wasmtime::Linker::new(engine);
            wasmtime_wasi::add_to_linker(&mut linker, |(_, wasi)| wasi)?;
            imports::add_functions_to_linker(&mut linker, |(host, _)| host)?;
            Ok(linker)
        },
    )?;

    // Test that the import instance called the functions we made available with
    // the expected arguments.

    assert!(
        store.data().0.f1_called,
        "top-level JS imported and called `f1`",
    );

    assert_eq!(
        store.data().0.f2_arg,
        42,
        "f2 should have been called with 42",
    );

    assert_eq!(store.data().0.f3_a, 0);
    assert_eq!(store.data().0.f3_b, u32::MAX);

    assert!(
        store.data().0.f4_called,
        "the top-level JS imported and called `f4`",
    );

    assert!(
        store.data().0.f5_called,
        "the top-level JS imported and called `f5`"
    );

    assert_eq!(store.data().0.f6_a, 100);
    assert_eq!(store.data().0.f6_b, 200);
    assert_eq!(store.data().0.f6_c, 300);

    // Test that the export instance behaves as we expect it to.

    let export_instance =
        exports::functions::Functions::new(&mut store, &export_instance, |_| todo!())
            .context("should create `Functions` object from instance")?;

    export_instance
        .f1(&mut store)
        .context("calling the `f1` export should succeed")?;

    export_instance
        .f2(&mut store, 42)
        .context("calling the `f2` export should succeed")?;

    export_instance
        .f3(&mut store, 0, u32::MAX)
        .context("calling the `f3` export should succeed")?;

    let a = export_instance
        .f4(&mut store)
        .context("calling the `f4` export should succeed")?;
    assert_eq!(a, 1337);

    let (a, b) = export_instance
        .f5(&mut store)
        .context("calling the `f5` export should succeed")?;
    assert_eq!(a, 1);
    assert_eq!(b, 2);

    let (a, b, c) = export_instance
        .f6(&mut store, 100, 200, 300)
        .context("calling the `f6` export should succeed")?;
    assert_eq!(a, 101);
    assert_eq!(b, 201);
    assert_eq!(c, 301);

    Ok(())
}
