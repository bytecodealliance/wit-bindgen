#[cfg(not(feature = "unchecked"))]
witx_bindgen_rust::import!("tests/host.witx");

#[cfg(feature = "unchecked")]
witx_bindgen_rust::import!({ paths: ["tests/host.witx"], unchecked });

use host::*;

use std::iter;

use crate::allocator;

pub fn run() {
    let _guard = allocator::guard();
    host_integers();
    host_floats();
    host_char();
    host_get_set();
    host_records();
    host_variants();
    host_legacy();
    host_lists();
    host_flavorful();
    host_handles();
    host_buffers();
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
    assert_eq!(list_result3(), ["hello,", "world!"]);
}

fn host_flavorful() {
    list_in_record1(ListInRecord1 {
        a: "list_in_record1",
    });
    assert_eq!(list_in_record2().a, "list_in_record2");

    assert_eq!(
        list_in_record3(ListInRecord3Param {
            a: "list_in_record3 input"
        })
        .a,
        "list_in_record3 output"
    );

    assert_eq!(
        list_in_record4(ListInAliasParam { a: "input4" }).a,
        "result4"
    );

    list_in_variant1(Some("foo"), Err("bar"), ListInVariant13::V0("baz"));
    assert_eq!(list_in_variant2(), Some("list_in_variant2".to_string()));
    assert_eq!(
        list_in_variant3(Some("input3")),
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

fn host_handles() {
    let s: HostState = host_state_create();
    assert_eq!(host_state_get(&s), 100);
    assert_eq!(host_state2_saw_close(), false);
    let s: HostState2 = host_state2_create();
    assert_eq!(host_state2_saw_close(), false);
    drop(s);
    assert_eq!(host_state2_saw_close(), true);

    let (_a, s2) = two_host_states(&host_state_create(), &host_state2_create());

    host_state2_param_record(HostStateParamRecord { a: &s2 });
    host_state2_param_tuple((&s2,));
    host_state2_param_option(Some(&s2));
    host_state2_param_option(None);
    host_state2_param_result(Ok(&s2));
    host_state2_param_result(Err(2));
    host_state2_param_variant(HostStateParamVariant::V0(&s2));
    host_state2_param_variant(HostStateParamVariant::V1(2));
    host_state2_param_list(&[]);
    host_state2_param_list(&[&s2]);
    host_state2_param_list(&[&s2, &s2]);

    drop(host_state2_result_record().a);
    drop(host_state2_result_tuple().0);
    drop(host_state2_result_option().unwrap());
    drop(host_state2_result_result().unwrap());
    drop(host_state2_result_variant());
    drop(host_state2_result_list());
}

fn host_buffers() {
    use witx_bindgen_rust::imports::{InBuffer, OutBuffer};

    let mut out = [0; 10];
    let n = buffer_u8(&[0u8], &mut out) as usize;
    assert_eq!(n, 3);
    assert_eq!(&out[..n], [1, 2, 3]);
    assert!(out[n..].iter().all(|x| *x == 0));

    let mut out = [0; 10];
    let n = buffer_u32(&[0], &mut out) as usize;
    assert_eq!(n, 3);
    assert_eq!(&out[..n], [1, 2, 3]);
    assert!(out[n..].iter().all(|x| *x == 0));

    let mut space1 = [0; 200];
    let mut space2 = [0; 200];

    assert_eq!(
        buffer_bool(
            &mut InBuffer::new(&mut space1, &mut iter::empty()),
            &mut OutBuffer::new(&mut space2)
        ),
        0
    );
    assert_eq!(
        buffer_string(
            &mut InBuffer::new(&mut space1, &mut iter::empty()),
            &mut OutBuffer::new(&mut space2)
        ),
        0
    );
    assert_eq!(
        buffer_list_bool(
            &mut InBuffer::new(&mut space1, &mut iter::empty()),
            &mut OutBuffer::new(&mut space2)
        ),
        0
    );

    let mut bools = [true, false, true].iter().copied();
    let mut out = OutBuffer::new(&mut space2);
    let n = buffer_bool(&mut InBuffer::new(&mut space1, &mut bools), &mut out);
    unsafe {
        assert_eq!(n, 3);
        assert_eq!(out.into_iter(3).collect::<Vec<_>>(), [false, true, false]);
    }

    let mut strings = ["foo", "bar", "baz"].iter().copied();
    let mut out = OutBuffer::new(&mut space2);
    let n = buffer_string(&mut InBuffer::new(&mut space1, &mut strings), &mut out);
    unsafe {
        assert_eq!(n, 3);
        assert_eq!(out.into_iter(3).collect::<Vec<_>>(), ["FOO", "BAR", "BAZ"]);
    }

    let a = &[true, false, true][..];
    let b = &[false, false][..];
    let list = [a, b];
    let mut lists = list.iter().copied();
    let mut out = OutBuffer::new(&mut space2);
    let n = buffer_list_bool(&mut InBuffer::new(&mut space1, &mut lists), &mut out);
    unsafe {
        assert_eq!(n, 2);
        assert_eq!(
            out.into_iter(2).collect::<Vec<_>>(),
            [vec![false, true, false], vec![true, true]]
        );
    }

    let a = [true, false, true, true, false];
    let mut bools = a.iter().copied();
    let mut b = InBuffer::new(&mut space2, &mut bools);
    let mut list = [&mut b];
    let mut buffers = &mut list.iter_mut().map(|b| &mut **b);
    buffer_buffer_bool(&mut InBuffer::new(&mut space1, &mut buffers));

    let mut bools = a.iter().copied();
    buffer_mutable1(&mut [&mut InBuffer::new(&mut space1, &mut bools)]);

    let n = buffer_mutable2(&mut [&mut space2]) as usize;
    assert_eq!(n, 4);
    assert_eq!(&space2[..n], [1, 2, 3, 4]);

    let mut out = OutBuffer::new(&mut space1);
    let n = buffer_mutable3(&mut [&mut out]);
    unsafe {
        assert_eq!(n, 3);
        assert_eq!(out.into_iter(3).collect::<Vec<_>>(), [false, true, false],);
    }
}

mod invalid {
    #[link(wasm_import_module = "host")]
    extern "C" {
        fn invert_bool(a: i32) -> i32;
        fn roundtrip_char(a: i32) -> i32;
        fn roundtrip_enum(a: i32) -> i32;
        fn host_state_get(a: i32) -> i32;
        fn host_state2_close(a: i32);
    }
    #[no_mangle]
    pub unsafe extern "C" fn invalid_bool() {
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
        roundtrip_char(0xd800);
    }

    #[no_mangle]
    pub unsafe extern "C" fn invalid_e1() {
        roundtrip_enum(400);
    }

    #[no_mangle]
    pub unsafe extern "C" fn invalid_handle() {
        host_state_get(100);
    }

    #[no_mangle]
    pub unsafe extern "C" fn invalid_handle_close() {
        host_state2_close(100);
    }
}
