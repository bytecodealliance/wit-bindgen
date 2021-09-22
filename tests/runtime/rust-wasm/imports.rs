#[cfg(not(feature = "unchecked"))]
witx_bindgen_rust::import!("tests/host.witx");

#[cfg(feature = "unchecked")]
witx_bindgen_rust::import!({ paths: ["tests/host.witx"], unchecked });

use crate::allocator;
use host::*;
use std::alloc::{self, Layout};
use std::iter;
use std::mem;
use std::ptr;

pub fn run() {
    let _guard = allocator::guard();
    host_integers();
    host_floats();
    host_char();
    host_get_set();
    host_records();
    host_variants();
    host_lists();
    host_flavorful();
    host_handles();
    host_buffers();
}

fn host_integers() {
    assert_eq!(multiple_results(), (4, 5));
}

fn host_floats() {}

fn host_char() {}

fn host_get_set() {}

fn host_records() {}

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

    let (a1, a2, a3, a4) = variant_zeros((Z1::B, Z2::B, Z3::B, Z4::B));
    assert!(matches!(a1, Z1::B));
    assert!(matches!(a2, Z2::B));
    assert!(matches!(a3, Z3::B));
    assert!(matches!(a4, Z4::B));

    variant_typedefs(None, false, Err(()));

    assert_eq!(
        variant_enums(true, Ok(()), MyErrno::Success),
        (false, Err(()), MyErrno::A)
    );
}

fn host_lists() {
    list_param(&[1, 2, 3, 4]);
    list_param2("foo");
    list_param3(&["foo", "bar", "baz"]);
    list_param4(&[&["foo", "bar"], &["baz"]]);
    assert_eq!(list_result(), [1, 2, 3, 4, 5]);
    assert_eq!(list_result2(), "hello!");
    assert_eq!(list_result3(), ["hello,", "world!"]);

    assert_eq!(string_roundtrip("x"), "x");
    assert_eq!(string_roundtrip(""), "");
    assert_eq!(string_roundtrip("hello"), "hello");
    assert_eq!(string_roundtrip("hello ⚑ world"), "hello ⚑ world");

    struct Unaligned<T: Copy> {
        alloc: *mut u8,
        _marker: std::marker::PhantomData<T>,
    }

    impl<T: Copy> Unaligned<T> {
        fn layout() -> Layout {
            Layout::from_size_align(2 * mem::size_of::<T>(), 8).unwrap()
        }

        fn new(data: T) -> Unaligned<T> {
            unsafe {
                let alloc = alloc::alloc(Self::layout());
                assert!(!alloc.is_null());
                ptr::write_unaligned(alloc.add(1).cast(), data);
                Unaligned {
                    alloc,
                    _marker: Default::default(),
                }
            }
        }

        fn as_slice(&self) -> *const [T] {
            unsafe { ptr::slice_from_raw_parts(self.alloc.add(1).cast(), 1) }
        }
    }

    impl<T: Copy> Drop for Unaligned<T> {
        fn drop(&mut self) {
            unsafe {
                alloc::dealloc(self.alloc, Self::layout());
            }
        }
    }

    unsafe {
        let u16s = Unaligned::new(1);
        let u32s = Unaligned::new(2);
        let u64s = Unaligned::new(3);
        let flag32s = Unaligned::new(FLAG32_B8);
        let flag64s = Unaligned::new(FLAG64_B9);
        let records = Unaligned::new(UnalignedRecord { a: 10, b: 11 });
        let f32s = Unaligned::new(100.0);
        let f64s = Unaligned::new(101.0);
        let strings = Unaligned::new("foo");
        let lists = Unaligned::new(&[102][..]);
        // Technically this is UB because we're creating safe slices from
        // unaligned pointers, but we're hoping that because we're just passing
        // off pointers to an import through a safe import we can get away with
        // this. If this ever becomes a problem we'll just need to call the raw
        // import with raw integers.
        unaligned_roundtrip1(
            &*u16s.as_slice(),
            &*u32s.as_slice(),
            &*u64s.as_slice(),
            &*flag32s.as_slice(),
            &*flag64s.as_slice(),
        );
        unaligned_roundtrip2(
            &*records.as_slice(),
            &*f32s.as_slice(),
            &*f64s.as_slice(),
            &*strings.as_slice(),
            &*lists.as_slice(),
        );
    }

    assert_eq!(
        list_minmax8(&[u8::MIN, u8::MAX], &[i8::MIN, i8::MAX]),
        (vec![u8::MIN, u8::MAX], vec![i8::MIN, i8::MAX]),
    );
    assert_eq!(
        list_minmax16(&[u16::MIN, u16::MAX], &[i16::MIN, i16::MAX]),
        (vec![u16::MIN, u16::MAX], vec![i16::MIN, i16::MAX]),
    );
    assert_eq!(
        list_minmax32(&[u32::MIN, u32::MAX], &[i32::MIN, i32::MAX]),
        (vec![u32::MIN, u32::MAX], vec![i32::MIN, i32::MAX]),
    );
    assert_eq!(
        list_minmax64(&[u64::MIN, u64::MAX], &[i64::MIN, i64::MAX]),
        (vec![u64::MIN, u64::MAX], vec![i64::MIN, i64::MAX]),
    );
    assert_eq!(
        list_minmax_float(
            &[f32::MIN, f32::MAX, f32::NEG_INFINITY, f32::INFINITY],
            &[f64::MIN, f64::MAX, f64::NEG_INFINITY, f64::INFINITY]
        ),
        (
            vec![f32::MIN, f32::MAX, f32::NEG_INFINITY, f32::INFINITY],
            vec![f64::MIN, f64::MAX, f64::NEG_INFINITY, f64::INFINITY],
        ),
    );
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

    let (a, b, c) = list_of_variants(
        &[true, false],
        &[Ok(()), Err(())],
        &[MyErrno::Success, MyErrno::A],
    );
    assert_eq!(a, [false, true]);
    assert_eq!(b, [Err(()), Ok(())]);
    assert_eq!(c, [MyErrno::A, MyErrno::B]);
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

    let md = Markdown2::create();
    md.append("red is the best color");
    assert_eq!(md.render(), "green is the best color");
}

fn host_buffers() {
    use witx_bindgen_rust::imports::{PullBuffer, PushBuffer};

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
            &mut PullBuffer::new(&mut space1, &mut iter::empty()),
            &mut PushBuffer::new(&mut space2)
        ),
        0
    );
    // assert_eq!(
    //     buffer_string(
    //         &mut PullBuffer::new(&mut space1, &mut iter::empty()),
    //         &mut PushBuffer::new(&mut space2)
    //     ),
    //     0
    // );
    // assert_eq!(
    //     buffer_list_bool(
    //         &mut PullBuffer::new(&mut space1, &mut iter::empty()),
    //         &mut PushBuffer::new(&mut space2)
    //     ),
    //     0
    // );

    let mut bools = [true, false, true].iter().copied();
    let mut out = PushBuffer::new(&mut space2);
    let n = buffer_bool(&mut PullBuffer::new(&mut space1, &mut bools), &mut out);
    unsafe {
        assert_eq!(n, 3);
        assert_eq!(out.into_iter(3).collect::<Vec<_>>(), [false, true, false]);
    }

    // let mut strings = ["foo", "bar", "baz"].iter().copied();
    // let mut out = PushBuffer::new(&mut space2);
    // let n = buffer_string(&mut PullBuffer::new(&mut space1, &mut strings), &mut out);
    // unsafe {
    //     assert_eq!(n, 3);
    //     assert_eq!(out.into_iter(3).collect::<Vec<_>>(), ["FOO", "BAR", "BAZ"]);
    // }

    // let a = &[true, false, true][..];
    // let b = &[false, false][..];
    // let list = [a, b];
    // let mut lists = list.iter().copied();
    // let mut out = PushBuffer::new(&mut space2);
    // let n = buffer_list_bool(&mut PullBuffer::new(&mut space1, &mut lists), &mut out);
    // unsafe {
    //     assert_eq!(n, 2);
    //     assert_eq!(
    //         out.into_iter(2).collect::<Vec<_>>(),
    //         [vec![false, true, false], vec![true, true]]
    //     );
    // }

    let a = [true, false, true, true, false];
    // let mut bools = a.iter().copied();
    // let mut b = PullBuffer::new(&mut space2, &mut bools);
    // let mut list = [&mut b];
    // let mut buffers = &mut list.iter_mut().map(|b| &mut **b);
    // buffer_buffer_bool(&mut PullBuffer::new(&mut space1, &mut buffers));

    let mut bools = a.iter().copied();
    buffer_mutable1(&mut [&mut PullBuffer::new(&mut space1, &mut bools)]);

    let n = buffer_mutable2(&mut [&mut space2]) as usize;
    assert_eq!(n, 4);
    assert_eq!(&space2[..n], [1, 2, 3, 4]);

    let mut out = PushBuffer::new(&mut space1);
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
    }
    #[link(wasm_import_module = "canonical_abi")]
    extern "C" {
        fn resource_drop_host_state2(a: i32);
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
        resource_drop_host_state2(100);
    }
}
