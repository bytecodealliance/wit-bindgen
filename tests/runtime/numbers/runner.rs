include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        use test::numbers::numbers::*;
        assert_eq!(roundtrip_u8(1), 1);
        assert_eq!(roundtrip_u8(u8::MIN), u8::MIN);
        assert_eq!(roundtrip_u8(u8::MAX), u8::MAX);

        assert_eq!(roundtrip_s8(1), 1);
        assert_eq!(roundtrip_s8(i8::MIN), i8::MIN);
        assert_eq!(roundtrip_s8(i8::MAX), i8::MAX);

        assert_eq!(roundtrip_u16(1), 1);
        assert_eq!(roundtrip_u16(u16::MIN), u16::MIN);
        assert_eq!(roundtrip_u16(u16::MAX), u16::MAX);

        assert_eq!(roundtrip_s16(1), 1);
        assert_eq!(roundtrip_s16(i16::MIN), i16::MIN);
        assert_eq!(roundtrip_s16(i16::MAX), i16::MAX);

        assert_eq!(roundtrip_u32(1), 1);
        assert_eq!(roundtrip_u32(u32::MIN), u32::MIN);
        assert_eq!(roundtrip_u32(u32::MAX), u32::MAX);

        assert_eq!(roundtrip_s32(1), 1);
        assert_eq!(roundtrip_s32(i32::MIN), i32::MIN);
        assert_eq!(roundtrip_s32(i32::MAX), i32::MAX);

        assert_eq!(roundtrip_u64(1), 1);
        assert_eq!(roundtrip_u64(u64::MIN), u64::MIN);
        assert_eq!(roundtrip_u64(u64::MAX), u64::MAX);

        assert_eq!(roundtrip_s64(1), 1);
        assert_eq!(roundtrip_s64(i64::MIN), i64::MIN);
        assert_eq!(roundtrip_s64(i64::MAX), i64::MAX);

        assert_eq!(roundtrip_f32(1.0), 1.0);
        assert_eq!(roundtrip_f32(f32::INFINITY), f32::INFINITY);
        assert_eq!(roundtrip_f32(f32::NEG_INFINITY), f32::NEG_INFINITY);
        assert!(roundtrip_f32(f32::NAN).is_nan());

        assert_eq!(roundtrip_f64(1.0), 1.0);
        assert_eq!(roundtrip_f64(f64::INFINITY), f64::INFINITY);
        assert_eq!(roundtrip_f64(f64::NEG_INFINITY), f64::NEG_INFINITY);
        assert!(roundtrip_f64(f64::NAN).is_nan());

        assert_eq!(roundtrip_char('a'), 'a');
        assert_eq!(roundtrip_char(' '), ' ');
        assert_eq!(roundtrip_char('🚩'), '🚩');

        set_scalar(2);
        assert_eq!(get_scalar(), 2);
        set_scalar(4);
        assert_eq!(get_scalar(), 4);
    }
}
