#![cfg(target_arch = "wasm32")]

witx_bindgen_rust::import!("tests/host.witx");

#[no_mangle]
pub extern "C" fn run_host_tests() {
    host_integers();
    host_floats();
    host_char();
    host_get_set();
    host_records();
    host_variants();
}

fn host_integers() {
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

    assert_eq!(roundtrip_usize(1), 1);
    assert_eq!(roundtrip_usize(usize::min_value()), usize::min_value());
    assert_eq!(roundtrip_usize(usize::max_value()), usize::max_value());
}

fn host_floats() {
    assert_eq!(roundtrip_f32(1.0), 1.0);
    assert_eq!(roundtrip_f32(f32::INFINITY), f32::INFINITY);
    assert_eq!(roundtrip_f32(f32::NEG_INFINITY), f32::NEG_INFINITY);
    assert!(roundtrip_f32(f32::NAN).is_nan());

    assert_eq!(roundtrip_f64(1.0), 1.0);
    assert_eq!(roundtrip_f64(f64::INFINITY), f64::INFINITY);
    assert_eq!(roundtrip_f64(f64::NEG_INFINITY), f64::NEG_INFINITY);
    assert!(roundtrip_f64(f64::NAN).is_nan());
}

fn host_char() {
    assert_eq!(roundtrip_char('a'), 'a');
    assert_eq!(roundtrip_char(' '), ' ');
    assert_eq!(roundtrip_char('🚩'), '🚩');
}

fn host_get_set() {
    set_scalar(2);
    assert_eq!(get_scalar(), 2);
    set_scalar(4);
    assert_eq!(get_scalar(), 4);
}

fn host_records() {
    assert_eq!(swap_tuple((1u8, 2u32)), (2u32, 1u8));
    assert_eq!(roundtrip_flags1(F1_A), F1_A);
    assert_eq!(roundtrip_flags1(0), 0);
    assert_eq!(roundtrip_flags1(F1_B), F1_B);
    assert_eq!(roundtrip_flags1(F1_A | F1_B), F1_A | F1_B);

    let r = roundtrip_record1(R1 { a: 8, b: 0 });
    assert_eq!(r.a, 8);
    assert_eq!(r.b, 0);

    let r = roundtrip_record1(R1 {
        a: 0,
        b: F1_A | F1_B,
    });
    assert_eq!(r.a, 0);
    assert_eq!(r.b, F1_A | F1_B);

    assert_eq!(tuple0(()), ());
    assert_eq!(tuple1((1,)), (1,));
}

fn host_variants() {
    assert_eq!(roundtrip_option(Some(1.0)), Some(1));
    assert_eq!(roundtrip_option(None), None);
    assert_eq!(roundtrip_option(Some(2.0)), Some(2));
    assert_eq!(roundtrip_result(Ok(2)), Ok(2.0));
    assert_eq!(roundtrip_result(Ok(4)), Ok(4.0));
    assert_eq!(roundtrip_result(Err(5.3)), Err(5));

    assert_eq!(roundtrip_enum(E1::A), E1::A);
    assert_eq!(roundtrip_enum(E1::B), E1::B);

    assert_eq!(invert_bool(true), false);
    assert_eq!(invert_bool(false), true);

    let (a1, a2, a3, a4, a5, a6) =
        variant_casts((C1::A(1), C2::A(2), C3::A(3), C4::A(4), C5::A(5), C6::A(6.0)));
    assert!(matches!(a1, C1::A(1)));
    assert!(matches!(a2, C2::A(2)));
    assert!(matches!(a3, C3::A(3)));
    assert!(matches!(a4, C4::A(4)));
    assert!(matches!(a5, C5::A(5)));
    assert!(matches!(a6, C6::A(6.0)));

    let (a1, a2, a3, a4, a5, a6) = variant_casts((
        C1::B(1),
        C2::B(2.0),
        C3::B(3.0),
        C4::B(4.0),
        C5::B(5.0),
        C6::B(6.0),
    ));
    assert!(matches!(a1, C1::B(1)));
    assert!(matches!(a2, C2::B(2.0)));
    assert!(matches!(a3, C3::B(3.0)));
    assert!(matches!(a4, C4::B(4.0)));
    assert!(matches!(a5, C5::B(5.0)));
    assert!(matches!(a6, C6::B(6.0)));
}

#[no_mangle]
pub unsafe extern "C" fn invalid_bool() {
    #[link(wasm_import_module = "host")]
    extern "C" {
        fn invert_bool(a: i32) -> i32;
    }
    invert_bool(2);
}

macro_rules! invalid_int {
    ($($name:ident $import:ident)*) => ($(
        #[no_mangle]
        pub unsafe extern "C" fn $name() {
            #[link(wasm_import_module = "host")]
            extern "C" {
                fn $import(a: i32) -> i32;
            }
            $import(i32::max_value());
        }
    )*)
}

invalid_int! {
    invalid_u8 roundtrip_u8
    invalid_s8 roundtrip_s8
    invalid_u16 roundtrip_u16
    invalid_s16 roundtrip_s16
}

#[no_mangle]
pub unsafe extern "C" fn invalid_char() {
    #[link(wasm_import_module = "host")]
    extern "C" {
        fn roundtrip_char(a: i32) -> i32;
    }
    roundtrip_char(0xd800);
}

#[no_mangle]
pub unsafe extern "C" fn invalid_e1() {
    #[link(wasm_import_module = "host")]
    extern "C" {
        fn roundtrip_enum(a: i32) -> i32;
    }
    roundtrip_enum(400);
}
