use anyhow::Result;
use wasmtime::Store;

wasmtime::component::bindgen!("world" in "tests/runtime/records");

#[derive(Default)]
pub struct MyImports;

impl imports::Imports for MyImports {
    fn multiple_results(&mut self) -> Result<(u8, u16)> {
        Ok((4, 5))
    }

    fn swap_tuple(&mut self, a: (u8, u32)) -> Result<(u32, u8)> {
        Ok((a.1, a.0))
    }

    fn roundtrip_flags1(&mut self, a: imports::F1) -> Result<imports::F1> {
        drop(format!("{:?}", a));
        drop(a & imports::F1::all());
        Ok(a)
    }

    fn roundtrip_flags2(&mut self, a: imports::F2) -> Result<imports::F2> {
        Ok(a)
    }

    fn roundtrip_flags3(
        &mut self,
        a: imports::Flag8,
        b: imports::Flag16,
        c: imports::Flag32,
        d: imports::Flag64,
    ) -> Result<(
        imports::Flag8,
        imports::Flag16,
        imports::Flag32,
        imports::Flag64,
    )> {
        Ok((a, b, c, d))
    }

    fn roundtrip_record1(&mut self, a: imports::R1) -> Result<imports::R1> {
        drop(format!("{:?}", a));
        Ok(a)
    }

    fn tuple0(&mut self, _: ()) -> Result<()> {
        Ok(())
    }

    fn tuple1(&mut self, a: (u8,)) -> Result<(u8,)> {
        Ok((a.0,))
    }
}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "records",
        |linker| Records::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| Records::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(exports: Records, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    use exports::*;

    exports.call_test_imports(&mut *store)?;
    let exports = exports.exports();
    assert_eq!(exports.call_multiple_results(&mut *store,)?, (100, 200));
    assert_eq!(
        exports.call_swap_tuple(&mut *store, (1u8, 2u32))?,
        (2u32, 1u8)
    );
    assert_eq!(exports.call_roundtrip_flags1(&mut *store, F1::A)?, F1::A);
    assert_eq!(
        exports.call_roundtrip_flags1(&mut *store, F1::empty())?,
        F1::empty()
    );
    assert_eq!(exports.call_roundtrip_flags1(&mut *store, F1::B)?, F1::B);
    assert_eq!(
        exports.call_roundtrip_flags1(&mut *store, F1::A | F1::B)?,
        F1::A | F1::B
    );

    assert_eq!(exports.call_roundtrip_flags2(&mut *store, F2::C)?, F2::C);
    assert_eq!(
        exports.call_roundtrip_flags2(&mut *store, F2::empty())?,
        F2::empty()
    );
    assert_eq!(exports.call_roundtrip_flags2(&mut *store, F2::D)?, F2::D);
    assert_eq!(
        exports.call_roundtrip_flags2(&mut *store, F2::C | F2::E)?,
        F2::C | F2::E
    );

    let r = exports.call_roundtrip_record1(
        &mut *store,
        R1 {
            a: 8,
            b: F1::empty(),
        },
    )?;
    assert_eq!(r.a, 8);
    assert_eq!(r.b, F1::empty());

    let r = exports.call_roundtrip_record1(
        &mut *store,
        R1 {
            a: 0,
            b: F1::A | F1::B,
        },
    )?;
    assert_eq!(r.a, 0);
    assert_eq!(r.b, F1::A | F1::B);

    assert_eq!(exports.call_tuple0(&mut *store, ())?, ());
    assert_eq!(exports.call_tuple1(&mut *store, (1,))?, (1,));
    Ok(())
}
