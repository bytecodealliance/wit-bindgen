use anyhow::Result;

wit_bindgen_wasmtime::export!("../../tests/runtime/numbers/imports.wit");

#[derive(Default)]
pub struct MyImports {
    scalar: u32,
}

impl imports::Imports for MyImports {
    fn roundtrip_u8(&mut self, val: u8) -> u8 {
        val
    }

    fn roundtrip_s8(&mut self, val: i8) -> i8 {
        val
    }

    fn roundtrip_u16(&mut self, val: u16) -> u16 {
        val
    }

    fn roundtrip_s16(&mut self, val: i16) -> i16 {
        val
    }

    fn roundtrip_u32(&mut self, val: u32) -> u32 {
        val
    }

    fn roundtrip_s32(&mut self, val: i32) -> i32 {
        val
    }

    fn roundtrip_u64(&mut self, val: u64) -> u64 {
        val
    }

    fn roundtrip_s64(&mut self, val: i64) -> i64 {
        val
    }

    fn roundtrip_float32(&mut self, val: f32) -> f32 {
        val
    }

    fn roundtrip_float64(&mut self, val: f64) -> f64 {
        val
    }

    fn roundtrip_char(&mut self, val: char) -> char {
        val
    }

    fn set_scalar(&mut self, val: u32) {
        self.scalar = val;
    }

    fn get_scalar(&mut self) -> u32 {
        self.scalar
    }
}

wit_bindgen_wasmtime::import!("../../tests/runtime/numbers/exports.wit");

fn run(wasm: &str) -> Result<()> {
    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| imports::add_to_linker(linker, |cx| -> &mut MyImports { &mut cx.imports }),
        |store, module, linker| {
            exports::Exports::instantiate(store, module, linker, |cx| &mut cx.exports)
        },
    )?;

    exports.test_imports(&mut store)?;
    assert_eq!(exports.roundtrip_u8(&mut store, 1)?, 1);
    assert_eq!(
        exports.roundtrip_u8(&mut store, u8::min_value())?,
        u8::min_value()
    );
    assert_eq!(
        exports.roundtrip_u8(&mut store, u8::max_value())?,
        u8::max_value()
    );

    assert_eq!(exports.roundtrip_s8(&mut store, 1)?, 1);
    assert_eq!(
        exports.roundtrip_s8(&mut store, i8::min_value())?,
        i8::min_value()
    );
    assert_eq!(
        exports.roundtrip_s8(&mut store, i8::max_value())?,
        i8::max_value()
    );

    assert_eq!(exports.roundtrip_u16(&mut store, 1)?, 1);
    assert_eq!(
        exports.roundtrip_u16(&mut store, u16::min_value())?,
        u16::min_value()
    );
    assert_eq!(
        exports.roundtrip_u16(&mut store, u16::max_value())?,
        u16::max_value()
    );

    assert_eq!(exports.roundtrip_s16(&mut store, 1)?, 1);
    assert_eq!(
        exports.roundtrip_s16(&mut store, i16::min_value())?,
        i16::min_value()
    );
    assert_eq!(
        exports.roundtrip_s16(&mut store, i16::max_value())?,
        i16::max_value()
    );

    assert_eq!(exports.roundtrip_u32(&mut store, 1)?, 1);
    assert_eq!(
        exports.roundtrip_u32(&mut store, u32::min_value())?,
        u32::min_value()
    );
    assert_eq!(
        exports.roundtrip_u32(&mut store, u32::max_value())?,
        u32::max_value()
    );

    assert_eq!(exports.roundtrip_s32(&mut store, 1)?, 1);
    assert_eq!(
        exports.roundtrip_s32(&mut store, i32::min_value())?,
        i32::min_value()
    );
    assert_eq!(
        exports.roundtrip_s32(&mut store, i32::max_value())?,
        i32::max_value()
    );

    assert_eq!(exports.roundtrip_u64(&mut store, 1)?, 1);
    assert_eq!(
        exports.roundtrip_u64(&mut store, u64::min_value())?,
        u64::min_value()
    );
    assert_eq!(
        exports.roundtrip_u64(&mut store, u64::max_value())?,
        u64::max_value()
    );

    assert_eq!(exports.roundtrip_s64(&mut store, 1)?, 1);
    assert_eq!(
        exports.roundtrip_s64(&mut store, i64::min_value())?,
        i64::min_value()
    );
    assert_eq!(
        exports.roundtrip_s64(&mut store, i64::max_value())?,
        i64::max_value()
    );

    assert_eq!(exports.roundtrip_float32(&mut store, 1.0)?, 1.0);
    assert_eq!(
        exports.roundtrip_float32(&mut store, f32::INFINITY)?,
        f32::INFINITY
    );
    assert_eq!(
        exports.roundtrip_float32(&mut store, f32::NEG_INFINITY)?,
        f32::NEG_INFINITY
    );
    assert!(exports.roundtrip_float32(&mut store, f32::NAN)?.is_nan());

    assert_eq!(exports.roundtrip_float64(&mut store, 1.0)?, 1.0);
    assert_eq!(
        exports.roundtrip_float64(&mut store, f64::INFINITY)?,
        f64::INFINITY
    );
    assert_eq!(
        exports.roundtrip_float64(&mut store, f64::NEG_INFINITY)?,
        f64::NEG_INFINITY
    );
    assert!(exports.roundtrip_float64(&mut store, f64::NAN)?.is_nan());

    assert_eq!(exports.roundtrip_char(&mut store, 'a')?, 'a');
    assert_eq!(exports.roundtrip_char(&mut store, ' ')?, ' ');
    assert_eq!(exports.roundtrip_char(&mut store, '🚩')?, '🚩');

    exports.set_scalar(&mut store, 2)?;
    assert_eq!(exports.get_scalar(&mut store)?, 2);
    exports.set_scalar(&mut store, 4)?;
    assert_eq!(exports.get_scalar(&mut store)?, 4);

    Ok(())
}
