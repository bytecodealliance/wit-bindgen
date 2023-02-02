use anyhow::Result;
use wasmtime::Store;

wasmtime::component::bindgen!("world" in "tests/runtime/numbers");

#[derive(Default)]
pub struct MyImports {
    scalar: u32,
}

impl imports::Imports for MyImports {
    fn roundtrip_u8(&mut self, val: u8) -> Result<u8> {
        Ok(val)
    }

    fn roundtrip_s8(&mut self, val: i8) -> Result<i8> {
        Ok(val)
    }

    fn roundtrip_u16(&mut self, val: u16) -> Result<u16> {
        Ok(val)
    }

    fn roundtrip_s16(&mut self, val: i16) -> Result<i16> {
        Ok(val)
    }

    fn roundtrip_u32(&mut self, val: u32) -> Result<u32> {
        Ok(val)
    }

    fn roundtrip_s32(&mut self, val: i32) -> Result<i32> {
        Ok(val)
    }

    fn roundtrip_u64(&mut self, val: u64) -> Result<u64> {
        Ok(val)
    }

    fn roundtrip_s64(&mut self, val: i64) -> Result<i64> {
        Ok(val)
    }

    fn roundtrip_float32(&mut self, val: f32) -> Result<f32> {
        Ok(val)
    }

    fn roundtrip_float64(&mut self, val: f64) -> Result<f64> {
        Ok(val)
    }

    fn roundtrip_char(&mut self, val: char) -> Result<char> {
        Ok(val)
    }

    fn set_scalar(&mut self, val: u32) -> Result<()> {
        self.scalar = val;
        Ok(())
    }

    fn get_scalar(&mut self) -> Result<u32> {
        Ok(self.scalar)
    }
}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "numbers",
        |linker| Numbers::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| Numbers::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(exports: Numbers, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    exports.call_test_imports(&mut *store)?;
    let exports = exports.exports();
    assert_eq!(exports.call_roundtrip_u8(&mut *store, 1)?, 1);
    assert_eq!(
        exports.call_roundtrip_u8(&mut *store, u8::min_value())?,
        u8::min_value()
    );
    assert_eq!(
        exports.call_roundtrip_u8(&mut *store, u8::max_value())?,
        u8::max_value()
    );

    assert_eq!(exports.call_roundtrip_s8(&mut *store, 1)?, 1);
    assert_eq!(
        exports.call_roundtrip_s8(&mut *store, i8::min_value())?,
        i8::min_value()
    );
    assert_eq!(
        exports.call_roundtrip_s8(&mut *store, i8::max_value())?,
        i8::max_value()
    );

    assert_eq!(exports.call_roundtrip_u16(&mut *store, 1)?, 1);
    assert_eq!(
        exports.call_roundtrip_u16(&mut *store, u16::min_value())?,
        u16::min_value()
    );
    assert_eq!(
        exports.call_roundtrip_u16(&mut *store, u16::max_value())?,
        u16::max_value()
    );

    assert_eq!(exports.call_roundtrip_s16(&mut *store, 1)?, 1);
    assert_eq!(
        exports.call_roundtrip_s16(&mut *store, i16::min_value())?,
        i16::min_value()
    );
    assert_eq!(
        exports.call_roundtrip_s16(&mut *store, i16::max_value())?,
        i16::max_value()
    );

    assert_eq!(exports.call_roundtrip_u32(&mut *store, 1)?, 1);
    assert_eq!(
        exports.call_roundtrip_u32(&mut *store, u32::min_value())?,
        u32::min_value()
    );
    assert_eq!(
        exports.call_roundtrip_u32(&mut *store, u32::max_value())?,
        u32::max_value()
    );

    assert_eq!(exports.call_roundtrip_s32(&mut *store, 1)?, 1);
    assert_eq!(
        exports.call_roundtrip_s32(&mut *store, i32::min_value())?,
        i32::min_value()
    );
    assert_eq!(
        exports.call_roundtrip_s32(&mut *store, i32::max_value())?,
        i32::max_value()
    );

    assert_eq!(exports.call_roundtrip_u64(&mut *store, 1)?, 1);
    assert_eq!(
        exports.call_roundtrip_u64(&mut *store, u64::min_value())?,
        u64::min_value()
    );
    assert_eq!(
        exports.call_roundtrip_u64(&mut *store, u64::max_value())?,
        u64::max_value()
    );

    assert_eq!(exports.call_roundtrip_s64(&mut *store, 1)?, 1);
    assert_eq!(
        exports.call_roundtrip_s64(&mut *store, i64::min_value())?,
        i64::min_value()
    );
    assert_eq!(
        exports.call_roundtrip_s64(&mut *store, i64::max_value())?,
        i64::max_value()
    );

    assert_eq!(exports.call_roundtrip_float32(&mut *store, 1.0)?, 1.0);
    assert_eq!(
        exports.call_roundtrip_float32(&mut *store, f32::INFINITY)?,
        f32::INFINITY
    );
    assert_eq!(
        exports.call_roundtrip_float32(&mut *store, f32::NEG_INFINITY)?,
        f32::NEG_INFINITY
    );
    assert!(exports
        .call_roundtrip_float32(&mut *store, f32::NAN)?
        .is_nan());

    assert_eq!(exports.call_roundtrip_float64(&mut *store, 1.0)?, 1.0);
    assert_eq!(
        exports.call_roundtrip_float64(&mut *store, f64::INFINITY)?,
        f64::INFINITY
    );
    assert_eq!(
        exports.call_roundtrip_float64(&mut *store, f64::NEG_INFINITY)?,
        f64::NEG_INFINITY
    );
    assert!(exports
        .call_roundtrip_float64(&mut *store, f64::NAN)?
        .is_nan());

    assert_eq!(exports.call_roundtrip_char(&mut *store, 'a')?, 'a');
    assert_eq!(exports.call_roundtrip_char(&mut *store, ' ')?, ' ');
    assert_eq!(exports.call_roundtrip_char(&mut *store, '🚩')?, '🚩');

    exports.call_set_scalar(&mut *store, 2)?;
    assert_eq!(exports.call_get_scalar(&mut *store)?, 2);
    exports.call_set_scalar(&mut *store, 4)?;
    assert_eq!(exports.call_get_scalar(&mut *store)?, 4);

    Ok(())
}
