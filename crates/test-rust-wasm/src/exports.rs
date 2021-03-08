#[cfg(not(feature = "unchecked"))]
witx_bindgen_rust::export!("tests/wasm.witx");

#[cfg(feature = "unchecked")]
witx_bindgen_rust::export!({ paths: ["tests/wasm.witx"], unchecked });

struct MyWasm;

fn wasm() -> &'static impl Wasm {
    &MyWasm
}

impl Wasm for MyWasm {
    fn run_import_tests(&self) {
        crate::imports::run();
    }

    fn roundtrip_u8(&self, a: u8) -> u8 {
        a
    }

    fn roundtrip_s8(&self, a: i8) -> i8 {
        a
    }

    fn roundtrip_u16(&self, a: u16) -> u16 {
        a
    }

    fn roundtrip_s16(&self, a: i16) -> i16 {
        a
    }

    fn roundtrip_u32(&self, a: u32) -> u32 {
        a
    }

    fn roundtrip_s32(&self, a: i32) -> i32 {
        a
    }

    fn roundtrip_u64(&self, a: u64) -> u64 {
        a
    }

    fn roundtrip_s64(&self, a: i64) -> i64 {
        a
    }

    fn roundtrip_f32(&self, a: f32) -> f32 {
        a
    }

    fn roundtrip_f64(&self, a: f64) -> f64 {
        a
    }

    fn roundtrip_char(&self, a: char) -> char {
        a
    }

    fn multiple_results(&self) -> (u8, u16) {
        (100, 200)
    }
}
