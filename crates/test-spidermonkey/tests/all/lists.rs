use anyhow::Context;

pub mod imports {
    use witx_bindgen_wasmtime::Le;

    witx_bindgen_wasmtime::import!("crates/test-spidermonkey/tests/lists.witx");
    pub use lists::add_lists_to_linker;

    #[derive(Default)]
    pub struct Host {
        pub f1_l: Vec<u32>,
        pub f2_called: bool,
        pub f3_a: Vec<u32>,
        pub f3_b: Vec<u32>,
        pub f4_l: Vec<Vec<u32>>,
    }

    impl lists::Lists for Host {
        fn f1(&mut self, l: &[Le<u32>]) {
            self.f1_l = l.iter().map(|le| le.get()).collect();
        }

        fn f2(&mut self) -> Vec<u32> {
            self.f2_called = true;
            vec![1, 2, 3]
        }

        fn f3(&mut self, a: &[Le<u32>], b: &[Le<u32>]) -> (Vec<u32>, Vec<u32>) {
            self.f3_a = a.iter().map(|le| le.get()).collect();
            self.f3_b = b.iter().map(|le| le.get()).collect();
            (vec![], vec![1, 2, 3])
        }

        fn f4(&mut self, l: Vec<&[Le<u32>]>) -> Vec<Vec<u32>> {
            self.f4_l = l
                .into_iter()
                .map(|xs| xs.iter().map(|le| le.get()).collect())
                .collect();
            vec![vec![], vec![4], vec![5, 6]]
        }
    }
}

pub mod exports {
    witx_bindgen_wasmtime::export!("crates/test-spidermonkey/tests/lists.witx");
}

#[test]
fn test() -> anyhow::Result<()> {
    let (mut store, _export_linker, _export_module, export_instance) = test_spidermonkey::run_test(
        "./tests/lists.witx",
        (
            imports::Host::default(),
            wasmtime_wasi::sync::WasiCtxBuilder::new()
                .inherit_stdio()
                .build(),
        ),
        |engine| {
            let mut linker = wasmtime::Linker::new(engine);
            wasmtime_wasi::add_to_linker(&mut linker, |(_, wasi)| wasi)?;
            imports::add_lists_to_linker(&mut linker, |(host, _)| host)?;
            Ok(linker)
        },
    )?;

    // Test that the import instance called the functions we made available with
    // the expected arguments.

    assert_eq!(store.data().0.f1_l, vec![1, 2, 3]);

    assert!(store.data().0.f2_called);

    assert_eq!(store.data().0.f3_a, vec![]);
    assert_eq!(store.data().0.f3_b, vec![1, 2, 3]);

    assert_eq!(store.data().0.f4_l, vec![vec![], vec![1], vec![2, 3]]);

    // Test that the export instance behaves as we expect it to.

    let export_instance = exports::lists::Lists::new(&mut store, &export_instance, |_| todo!())
        .context("should create `Lists` object from instance")?;

    export_instance
        .f1(&mut store, &[1, 2, 3])
        .context("calling the `f1` export should succeed")?;

    let l = export_instance
        .f2(&mut store)
        .context("calling the `f2` export should succeed")?;
    assert_eq!(l, vec![1, 2, 3]);

    let (a, b) = export_instance
        .f3(&mut store, &[], &[1, 2, 3])
        .context("calling the `f3` export should succeed")?;
    assert_eq!(a, vec![]);
    assert_eq!(b, vec![1, 2, 3]);

    let l = export_instance
        .f4(&mut store, &[&[], &[1], &[2, 3]])
        .context("calling the `f4` export should succeed")?;
    assert_eq!(l, vec![vec![], vec![4], vec![5, 6]]);

    Ok(())
}
