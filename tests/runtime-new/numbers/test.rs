include!(env!("BINDINGS"));

struct Component;

export!(Component);

use std::sync::atomic::{AtomicU32, Ordering::SeqCst};

static SCALAR: AtomicU32 = AtomicU32::new(0);

impl exports::test::numbers::numbers::Guest for Component {
    fn roundtrip_u8(a: u8) -> u8 {
        a
    }

    fn roundtrip_s8(a: i8) -> i8 {
        a
    }

    fn roundtrip_u16(a: u16) -> u16 {
        a
    }

    fn roundtrip_s16(a: i16) -> i16 {
        a
    }

    fn roundtrip_u32(a: u32) -> u32 {
        a
    }

    fn roundtrip_s32(a: i32) -> i32 {
        a
    }

    fn roundtrip_u64(a: u64) -> u64 {
        a
    }

    fn roundtrip_s64(a: i64) -> i64 {
        a
    }

    fn roundtrip_f32(a: f32) -> f32 {
        a
    }

    fn roundtrip_f64(a: f64) -> f64 {
        a
    }

    fn roundtrip_char(a: char) -> char {
        a
    }

    fn set_scalar(val: u32) {
        SCALAR.store(val, SeqCst)
    }

    fn get_scalar() -> u32 {
        SCALAR.load(SeqCst)
    }
}
