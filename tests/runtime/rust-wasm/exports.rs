#[cfg(not(feature = "unchecked"))]
witx_bindgen_rust::export!("tests/wasm.witx");

#[cfg(feature = "unchecked")]
witx_bindgen_rust::export!({ paths: ["tests/wasm.witx"], unchecked });

use wasm::*;
use witx_bindgen_rust::Handle;

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
// use witx_bindgen_rust::exports::{InBuffer, InBufferRaw, OutBuffer, OutBufferRaw};

struct Wasm;

pub struct WasmState(u32);

pub struct WasmState2(u32);

impl wasm::Wasm for Wasm {
    fn allocated_bytes() -> u32 {
        crate::allocator::get() as u32
    }

    fn run_import_tests() {
        crate::imports::run();
    }

    fn list_in_record1(ty: ListInRecord1) {
        assert_eq!(ty.a, "list_in_record1");
    }

    fn list_in_record2() -> ListInRecord2 {
        ListInRecord2 {
            a: "list_in_record2".to_string(),
        }
    }

    fn list_in_record3(a: ListInRecord3) -> ListInRecord3 {
        assert_eq!(a.a, "list_in_record3 input");
        ListInRecord3 {
            a: "list_in_record3 output".to_string(),
        }
    }

    fn list_in_record4(a: ListInAlias) -> ListInAlias {
        assert_eq!(a.a, "input4");
        ListInRecord4 {
            a: "result4".to_string(),
        }
    }

    fn list_in_variant1(a: ListInVariant11, b: ListInVariant12, c: ListInVariant13) {
        assert_eq!(a.unwrap(), "foo");
        assert_eq!(b.unwrap_err(), "bar");
        match c {
            ListInVariant13::V0(s) => assert_eq!(s, "baz"),
            ListInVariant13::V1(_) => panic!(),
        }
    }

    fn list_in_variant2() -> Option<String> {
        Some("list_in_variant2".to_string())
    }

    fn list_in_variant3(a: ListInVariant3) -> Option<String> {
        assert_eq!(a.unwrap(), "input3");
        Some("output3".to_string())
    }

    fn errno_result() -> Result<(), MyErrno> {
        MyErrno::A.to_string();
        format!("{:?}", MyErrno::A);
        fn assert_error<T: std::error::Error>() {}
        assert_error::<MyErrno>();
        Err(MyErrno::B)
    }

    fn list_typedefs(a: ListTypedef, b: ListTypedef3) -> (ListTypedef2, ListTypedef3) {
        assert_eq!(a, "typedef1");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0], "typedef2");
        (b"typedef3".to_vec(), vec!["typedef4".to_string()])
    }

    // fn buffer_u8( in_: InBufferRaw<'_, u8>, out: OutBufferRaw<'_, u8>) -> u32 {
    //     assert_eq!(in_.len(), 1);
    //     let mut input = [0];
    //     in_.copy(&mut input);
    //     assert_eq!(input, [0]);

    //     assert_eq!(out.capacity(), 10);
    //     out.write(&[1, 2, 3]);
    //     3
    // }

    // fn buffer_u32( in_: InBufferRaw<'_, u32>, out: OutBufferRaw<'_, u32>) -> u32 {
    //     assert_eq!(in_.len(), 1);
    //     let mut input = [0];
    //     in_.copy(&mut input);
    //     assert_eq!(input, [0]);

    //     assert_eq!(out.capacity(), 10);
    //     out.write(&[1, 2, 3]);
    //     3
    // }

    // fn buffer_bool( in_: InBuffer<'_, bool>, out: OutBuffer<'_, bool>) -> u32 {
    //     assert!(in_.len() <= out.capacity());
    //     let len = in_.len();
    //     let mut storage = vec![0; in_.len() * in_.element_size()];
    //     let items = in_.iter(&mut storage).map(|b| !b).collect::<Vec<_>>();
    //     out.write(&mut storage, items.into_iter());
    //     len as u32
    // }

    // fn buffer_string( in_: InBuffer<'_, String>, out: OutBuffer<'_, String>) -> u32 {
    //     assert!(in_.len() <= out.capacity());
    //     let len = in_.len();
    //     let mut storage = vec![0; in_.len() * in_.element_size()];
    //     let items = in_
    //         .iter(&mut storage)
    //         .map(|s| s.to_uppercase())
    //         .collect::<Vec<_>>();
    //     out.write(&mut storage, items.into_iter());
    //     len as u32
    // }

    // fn buffer_list_bool( in_: InBuffer<'_, Vec<bool>>, out: OutBuffer<'_, Vec<bool>>) -> u32 {
    //     assert!(in_.len() <= out.capacity());
    //     let len = in_.len();
    //     let mut storage = vec![0; in_.len() * in_.element_size()];
    //     let items = in_
    //         .iter(&mut storage)
    //         .map(|s| s.into_iter().map(|b| !b).collect::<Vec<_>>())
    //         .collect::<Vec<_>>();
    //     out.write(&mut storage, items.into_iter());
    //     len as u32
    // }

    // // fn buffer_buffer_bool( in_: InBuffer<'_, InBuffer<'_, bool>>) {
    // //     assert_eq!(in_.len(), 1);
    // //     let mut storage = vec![0; in_.len() * in_.element_size()];
    // //     let buf = in_.iter(&mut storage).next().unwrap();
    // //     assert_eq!(buf.len(), 5);
    // //     let mut storage2 = vec![0; buf.len() * buf.element_size()];
    // //     assert_eq!(
    // //         buf.iter(&mut storage2).collect::<Vec<bool>>(),
    // //         [true, false, true, true, false]
    // //     );
    // // }

    // fn buffer_mutable1( a: Vec<InBuffer<'_, bool>>) {
    //     assert_eq!(a.len(), 1);
    //     assert_eq!(a[0].len(), 5);
    //     let mut storage = vec![0; a[0].len() * a[0].element_size()];
    //     assert_eq!(
    //         a[0].iter(&mut storage).collect::<Vec<_>>(),
    //         [true, false, true, true, false]
    //     );
    // }

    // fn buffer_mutable2( a: Vec<OutBufferRaw<'_, u8>>) -> u32 {
    //     assert_eq!(a.len(), 1);
    //     assert!(a[0].capacity() > 4);
    //     a[0].write(&[1, 2, 3, 4]);
    //     return 4;
    // }

    // fn buffer_mutable3( a: Vec<OutBuffer<'_, bool>>) -> u32 {
    //     assert_eq!(a.len(), 1);
    //     assert!(a[0].capacity() > 3);
    //     let mut storage = [0; 200];
    //     a[0].write(&mut storage, [false, true, false].iter().copied());
    //     return 3;
    // }

    // fn buffer_in_record( _: BufferInRecord<'_>) {}
    // fn buffer_typedef(
    //
    //     _: ParamInBufferU8<'_>,
    //     _: ParamOutBufferU8<'_>,
    //     _: ParamInBufferBool<'_>,
    //     _: ParamOutBufferBool<'_>,
    // ) {
    // }
}
