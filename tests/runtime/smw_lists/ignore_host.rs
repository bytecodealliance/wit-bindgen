use anyhow::Context;
use wit_bindgen_wasmtime::Le;

wit_bindgen_wasmtime::export!("../../tests/runtime/smw_lists/imports.wit");

#[derive(Default)]
pub struct Host {
    pub f1_l: Vec<u32>,
    pub f2_called: bool,
    // pub f3_a: Vec<u32>,
    // pub f3_b: Vec<u32>,
    pub f4_l: Vec<Vec<u32>>,
}

impl imports::Imports for Host {
    fn f1(&mut self, l: &[Le<u32>]) {
        self.f1_l = l.iter().map(|le| le.get()).collect();
    }

    fn f2(&mut self) -> Vec<u32> {
        self.f2_called = true;
        vec![1, 2, 3]
    }

    // fn f3(&mut self, a: &[Le<u32>], b: &[Le<u32>]) -> (Vec<u32>, Vec<u32>) {
    //     self.f3_a = a.iter().map(|le| le.get()).collect();
    //     self.f3_b = b.iter().map(|le| le.get()).collect();
    //     (vec![], vec![1, 2, 3])
    // }

    fn f4(&mut self, l: Vec<&[Le<u32>]>) -> Vec<Vec<u32>> {
        self.f4_l = l
            .into_iter()
            .map(|xs| xs.iter().map(|le| le.get()).collect())
            .collect();
        vec![vec![], vec![4], vec![5, 6]]
    }
}

wit_bindgen_wasmtime::import!("../../tests/runtime/smw_lists/exports.wit");

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

    assert_eq!(store.data().imports.f1_l, vec![1, 2, 3]);

    assert!(store.data().imports.f2_called);

    // assert_eq!(store.data().imports.f3_a, vec![]);
    // assert_eq!(store.data().imports.f3_b, vec![1, 2, 3]);

    assert_eq!(store.data().imports.f4_l, vec![vec![], vec![1], vec![2, 3]]);

    // Test that the export instance behaves as we expect it to.

    exports
        .f1(&mut store, &[1, 2, 3])
        .context("calling the `f1` export should succeed")?;

    let l = exports
        .f2(&mut store)
        .context("calling the `f2` export should succeed")?;
    assert_eq!(l, vec![1, 2, 3]);

    // let (a, b) = exports
    //     .f3(&mut store, &[], &[1, 2, 3])
    //     .context("calling the `f3` export should succeed")?;
    // assert_eq!(a, vec![]);
    // assert_eq!(b, vec![1, 2, 3]);

    let l = exports
        .f4(&mut store, &[&[], &[1], &[2, 3]])
        .context("calling the `f4` export should succeed")?;
    assert_eq!(l, vec![vec![], vec![4], vec![5, 6]]);

    Ok(())
}
