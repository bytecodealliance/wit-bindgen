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

fn host_handles() {}

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
