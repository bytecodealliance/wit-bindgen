use anyhow::Result;
use wasmer::WasmerEnv;

wit_bindgen_wasmer::export!("./tests/runtime/records/imports.wit");

use imports::*;

#[derive(WasmerEnv, Clone)]
pub struct MyImports;

impl Imports for MyImports {
    fn multiple_results(&mut self) -> (u8, u16) {
        (4, 5)
    }

    fn swap_tuple(&mut self, a: (u8, u32)) -> (u32, u8) {
        (a.1, a.0)
    }

    fn roundtrip_flags1(&mut self, a: F1) -> F1 {
        drop(a.to_string());
        drop(format!("{:?}", a));
        drop(a & F1::all());
        a
    }

    fn roundtrip_flags2(&mut self, a: F2) -> F2 {
        a
    }

    fn roundtrip_flags3(
        &mut self,
        a: Flag8,
        b: Flag16,
        c: Flag32,
        d: Flag64,
    ) -> (Flag8, Flag16, Flag32, Flag64) {
        (a, b, c, d)
    }

    fn roundtrip_record1(&mut self, a: R1) -> R1 {
        drop(format!("{:?}", a));
        a
    }

    fn tuple0(&mut self, _: ()) {}

    fn tuple1(&mut self, a: (u8,)) -> (u8,) {
        (a.0,)
    }
}

wit_bindgen_wasmer::import!("./tests/runtime/records/exports.wit");

fn run(wasm: &str) -> Result<()> {
    use exports::*;

    let exports = crate::instantiate(
        wasm,
        |store, import_object| imports::add_to_imports(store, import_object, MyImports),
        |store, module, import_object| exports::Exports::instantiate(store, module, import_object),
    )?;

    exports.test_imports()?;
    assert_eq!(exports.multiple_results()?, (100, 200));
    assert_eq!(exports.swap_tuple((1u8, 2u32))?, (2u32, 1u8));
    assert_eq!(exports.roundtrip_flags1(F1::A)?, F1::A);
    assert_eq!(exports.roundtrip_flags1(F1::empty())?, F1::empty());
    assert_eq!(exports.roundtrip_flags1(F1::B)?, F1::B);
    assert_eq!(exports.roundtrip_flags1(F1::A | F1::B)?, F1::A | F1::B);

    assert_eq!(exports.roundtrip_flags2(F2::C)?, F2::C);
    assert_eq!(exports.roundtrip_flags2(F2::empty())?, F2::empty());
    assert_eq!(exports.roundtrip_flags2(F2::D)?, F2::D);
    assert_eq!(exports.roundtrip_flags2(F2::C | F2::E)?, F2::C | F2::E);

    let r = exports.roundtrip_record1(R1 {
        a: 8,
        b: F1::empty(),
    })?;
    assert_eq!(r.a, 8);
    assert_eq!(r.b, F1::empty());

    let r = exports.roundtrip_record1(R1 {
        a: 0,
        b: F1::A | F1::B,
    })?;
    assert_eq!(r.a, 0);
    assert_eq!(r.b, F1::A | F1::B);

    assert_eq!(exports.tuple0(())?, ());
    assert_eq!(exports.tuple1((1,))?, (1,));
    Ok(())
}
