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
}
