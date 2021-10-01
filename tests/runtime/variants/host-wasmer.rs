use anyhow::Result;
use wasmer::WasmerEnv;

wit_bindgen_wasmer::export!("./tests/runtime/variants/imports.wit");

use imports::*;

#[derive(WasmerEnv, Clone)]
pub struct MyImports;

impl Imports for MyImports {
    fn roundtrip_option(&mut self, a: Option<f32>) -> Option<u8> {
        a.map(|x| x as u8)
    }

    fn roundtrip_result(&mut self, a: Result<u32, f32>) -> Result<f64, u8> {
        match a {
            Ok(a) => Ok(a.into()),
            Err(b) => Err(b as u8),
        }
    }

    fn roundtrip_enum(&mut self, a: E1) -> E1 {
        assert_eq!(a, a);
        a
    }

    fn invert_bool(&mut self, a: bool) -> bool {
        !a
    }

    fn variant_casts(&mut self, a: Casts) -> Casts {
        a
    }

    fn variant_zeros(&mut self, a: Zeros) -> Zeros {
        a
    }

    fn variant_typedefs(&mut self, _: Option<u32>, _: bool, _: Result<u32, ()>) {}

    fn variant_enums(
        &mut self,
        a: bool,
        b: Result<(), ()>,
        c: MyErrno,
    ) -> (bool, Result<(), ()>, MyErrno) {
        assert_eq!(a, true);
        assert_eq!(b, Ok(()));
        assert_eq!(c, MyErrno::Success);
        (false, Err(()), MyErrno::A)
    }
}

wit_bindgen_wasmer::import!("./tests/runtime/variants/exports.wit");

fn run(wasm: &str) -> Result<()> {
    use exports::*;

    let exports = crate::instantiate(
        wasm,
        |store, import_object| imports::add_to_imports(store, import_object, MyImports),
        |store, module, import_object| exports::Exports::instantiate(store, module, import_object),
    )?;

    exports.test_imports()?;

    assert_eq!(exports.roundtrip_option(Some(1.0))?, Some(1));
    assert_eq!(exports.roundtrip_option(None)?, None);
    assert_eq!(exports.roundtrip_option(Some(2.0))?, Some(2));
    assert_eq!(exports.roundtrip_result(Ok(2))?, Ok(2.0));
    assert_eq!(exports.roundtrip_result(Ok(4))?, Ok(4.0));
    assert_eq!(exports.roundtrip_result(Err(5.3))?, Err(5));

    assert_eq!(exports.roundtrip_enum(E1::A)?, E1::A);
    assert_eq!(exports.roundtrip_enum(E1::B)?, E1::B);

    assert_eq!(exports.invert_bool(true)?, false);
    assert_eq!(exports.invert_bool(false)?, true);

    let (a1, a2, a3, a4, a5, a6) =
        exports.variant_casts((C1::A(1), C2::A(2), C3::A(3), C4::A(4), C5::A(5), C6::A(6.0)))?;
    assert!(matches!(a1, C1::A(1)));
    assert!(matches!(a2, C2::A(2)));
    assert!(matches!(a3, C3::A(3)));
    assert!(matches!(a4, C4::A(4)));
    assert!(matches!(a5, C5::A(5)));
    assert!(matches!(a6, C6::A(b) if b == 6.0));

    let (a1, a2, a3, a4, a5, a6) = exports.variant_casts((
        C1::B(1),
        C2::B(2.0),
        C3::B(3.0),
        C4::B(4.0),
        C5::B(5.0),
        C6::B(6.0),
    ))?;
    assert!(matches!(a1, C1::B(1)));
    assert!(matches!(a2, C2::B(b) if b == 2.0));
    assert!(matches!(a3, C3::B(b) if b == 3.0));
    assert!(matches!(a4, C4::B(b) if b == 4.0));
    assert!(matches!(a5, C5::B(b) if b == 5.0));
    assert!(matches!(a6, C6::B(b) if b == 6.0));

    let (a1, a2, a3, a4) = exports.variant_zeros((Z1::A(1), Z2::A(2), Z3::A(3.0), Z4::A(4.0)))?;
    assert!(matches!(a1, Z1::A(1)));
    assert!(matches!(a2, Z2::A(2)));
    assert!(matches!(a3, Z3::A(b) if b == 3.0));
    assert!(matches!(a4, Z4::A(b) if b == 4.0));

    exports.variant_typedefs(None, false, Err(()))?;

    Ok(())
}
