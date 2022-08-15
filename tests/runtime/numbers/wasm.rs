wit_bindgen_guest_rust::import!("../../tests/runtime/numbers/imports.wit");
wit_bindgen_guest_rust::export!("../../tests/runtime/numbers/exports.wit");

use imports::*;
use std::sync::atomic::{AtomicU32, Ordering::SeqCst};

struct Exports;

static SCALAR: AtomicU32 = AtomicU32::new(0);

impl exports::Exports for Exports {
    fn test_imports() {
        assert_eq!(roundtrip_u8(1), 1);
        assert_eq!(roundtrip_u8(u8::min_value()), u8::min_value());
        assert_eq!(roundtrip_u8(u8::max_value()), u8::max_value());

        assert_eq!(roundtrip_s8(1), 1);
        assert_eq!(roundtrip_s8(i8::min_value()), i8::min_value());
        assert_eq!(roundtrip_s8(i8::max_value()), i8::max_value());

        assert_eq!(roundtrip_u16(1), 1);
        assert_eq!(roundtrip_u16(u16::min_value()), u16::min_value());
        assert_eq!(roundtrip_u16(u16::max_value()), u16::max_value());

        assert_eq!(roundtrip_s16(1), 1);
        assert_eq!(roundtrip_s16(i16::min_value()), i16::min_value());
        assert_eq!(roundtrip_s16(i16::max_value()), i16::max_value());

        assert_eq!(roundtrip_u32(1), 1);
        assert_eq!(roundtrip_u32(u32::min_value()), u32::min_value());
        assert_eq!(roundtrip_u32(u32::max_value()), u32::max_value());

        assert_eq!(roundtrip_s32(1), 1);
        assert_eq!(roundtrip_s32(i32::min_value()), i32::min_value());
        assert_eq!(roundtrip_s32(i32::max_value()), i32::max_value());

        assert_eq!(roundtrip_u64(1), 1);
        assert_eq!(roundtrip_u64(u64::min_value()), u64::min_value());
        assert_eq!(roundtrip_u64(u64::max_value()), u64::max_value());

        assert_eq!(roundtrip_s64(1), 1);
        assert_eq!(roundtrip_s64(i64::min_value()), i64::min_value());
        assert_eq!(roundtrip_s64(i64::max_value()), i64::max_value());

        assert_eq!(roundtrip_float32(1.0), 1.0);
        assert_eq!(roundtrip_float32(f32::INFINITY), f32::INFINITY);
        assert_eq!(roundtrip_float32(f32::NEG_INFINITY), f32::NEG_INFINITY);
        assert!(roundtrip_float32(f32::NAN).is_nan());

        assert_eq!(roundtrip_float64(1.0), 1.0);
        assert_eq!(roundtrip_float64(f64::INFINITY), f64::INFINITY);
        assert_eq!(roundtrip_float64(f64::NEG_INFINITY), f64::NEG_INFINITY);
        assert!(roundtrip_float64(f64::NAN).is_nan());

        assert_eq!(roundtrip_char('a'), 'a');
        assert_eq!(roundtrip_char(' '), ' ');
        assert_eq!(roundtrip_char('🚩'), '🚩');

        set_scalar(2);
        assert_eq!(get_scalar(), 2);
        set_scalar(4);
        assert_eq!(get_scalar(), 4);
    }

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

    fn roundtrip_float32(a: f32) -> f32 {
        a
    }

    fn roundtrip_float64(a: f64) -> f64 {
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
