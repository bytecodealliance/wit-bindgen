#![cfg(target_arch = "wasm32")]

witx_bindgen_rust::import!("tests/host.witx");

// A small global allocator implementation which is intended to keep track of
// the number of allocated bytes to ensure that all our integration glue indeed
// manages memory correctly and doesn't leak anything.
mod allocator {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

    #[global_allocator]
    static ALLOC: A = A;

    static ALLOC_AMT: AtomicUsize = AtomicUsize::new(0);

    struct A;

    unsafe impl GlobalAlloc for A {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ptr = System.alloc(layout);
            if !ptr.is_null() {
                ALLOC_AMT.fetch_add(layout.size(), SeqCst);
            }
            return ptr;
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            ALLOC_AMT.fetch_sub(layout.size(), SeqCst);
            System.dealloc(ptr, layout)
        }
    }

    pub fn get() -> usize {
        ALLOC_AMT.load(SeqCst)
    }
}

#[no_mangle]
pub extern "C" fn run_host_tests() {
    let start = allocator::get();
    host_integers();
    host_floats();
    host_char();
    host_get_set();
    host_records();
    host_variants();
    host_legacy();
    host_lists();
    host_flavorful();
    assert_eq!(allocator::get(), start);
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

    assert_eq!(multiple_results(), (4, 5));
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

    assert_eq!(roundtrip_flags2(F2_C), F2_C);
    assert_eq!(roundtrip_flags2(0), 0);
    assert_eq!(roundtrip_flags2(F2_D), F2_D);
    assert_eq!(roundtrip_flags2(F2_C | F2_E), F2_C | F2_E);

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
    assert!(matches!(a6, C6::A(b) if b == 6.0));

    let (a1, a2, a3, a4, a5, a6) = variant_casts((
        C1::B(1),
        C2::B(2.0),
        C3::B(3.0),
        C4::B(4.0),
        C5::B(5.0),
        C6::B(6.0),
    ));
    assert!(matches!(a1, C1::B(1)));
    assert!(matches!(a2, C2::B(b) if b == 2.0));
    assert!(matches!(a3, C3::B(b) if b == 3.0));
    assert!(matches!(a4, C4::B(b) if b == 4.0));
    assert!(matches!(a5, C5::B(b) if b == 5.0));
    assert!(matches!(a6, C6::B(b) if b == 6.0));

    let (a1, a2, a3, a4) = variant_zeros((Z1::A(1), Z2::A(2), Z3::A(3.0), Z4::A(4.0)));
    assert!(matches!(a1, Z1::A(1)));
    assert!(matches!(a2, Z2::A(2)));
    assert!(matches!(a3, Z3::A(b) if b == 3.0));
    assert!(matches!(a4, Z4::A(b) if b == 4.0));

    variant_typedefs(None, false, Err(()));
}

fn host_legacy() {
    legacy_params(
        (1, 2),
        R1 {
            a: 0,
            b: F1_A | F1_B,
        },
        (1, 2, 3, 4, 5, 6, 7, 8, 9., 10.),
    );
    assert!(legacy_result(true).is_ok());
    assert!(legacy_result(false).is_err());
}

fn host_lists() {
    list_param(&[1, 2, 3, 4]);
    list_param2("foo");
    list_param3(&["foo", "bar", "baz"]);
    list_param4(&[&["foo", "bar"], &["baz"]]);
    assert_eq!(list_result(), [1, 2, 3, 4, 5]);
    assert_eq!(list_result2(), "hello!");
    // assert_eq!(list_result3(), ["hello,", "world!"]);
}

fn host_flavorful() {
    list_in_record1(&ListInRecord1 {
        a: "list_in_record1",
    });
    assert_eq!(list_in_record2().a, "list_in_record2");

    assert_eq!(
        list_in_record3(&ListInRecord3Param {
            a: "list_in_record3 input"
        })
        .a,
        "list_in_record3 output"
    );

    assert_eq!(
        list_in_record4(&ListInAliasParam { a: "input4" }).a,
        "result4"
    );

    list_in_variant1(&Some("foo"), &Err("bar"), &ListInVariant13::V0("baz"));
    assert_eq!(list_in_variant2(), Some("list_in_variant2".to_string()));
    assert_eq!(
        list_in_variant3(&Some("input3")),
        Some("output3".to_string())
    );

    assert!(errno_result().is_err());
    MyErrno::A.to_string();
    format!("{:?}", MyErrno::A);
    fn assert_error<T: std::error::Error>() {}
    assert_error::<MyErrno>();

    let (a, b) = list_typedefs("typedef1", &["typedef2"]);
    assert_eq!(a, b"typedef3");
    assert_eq!(b.len(), 1);
    assert_eq!(b[0], "typedef4");
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
