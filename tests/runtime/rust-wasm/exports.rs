#[cfg(not(feature = "unchecked"))]
witx_bindgen_rust::export!("tests/wasm.witx");

#[cfg(feature = "unchecked")]
witx_bindgen_rust::export!({ paths: ["tests/wasm.witx"], unchecked });

use wasm::*;
use witx_bindgen_rust::Handle;

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
// use witx_bindgen_rust::exports::{InBuffer, InBufferRaw, OutBuffer, OutBufferRaw};

static CLOSED: AtomicU32 = AtomicU32::new(0);

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

    fn roundtrip_option(a: Option<f32>) -> Option<u8> {
        a.map(|x| x as u8)
    }

    fn roundtrip_result(a: Result<u32, f32>) -> Result<f64, u8> {
        match a {
            Ok(a) => Ok(a.into()),
            Err(b) => Err(b as u8),
        }
    }

    fn roundtrip_enum(a: E1) -> E1 {
        assert_eq!(a, a);
        a
    }

    fn invert_bool(a: bool) -> bool {
        !a
    }

    fn variant_casts(a: Casts) -> Casts {
        a
    }

    fn variant_zeros(a: Zeros) -> Zeros {
        a
    }

    fn variant_typedefs(_: Option<u32>, _: bool, _: Result<u32, ()>) {}

    fn list_param(list: Vec<u8>) {
        assert_eq!(list, [1, 2, 3, 4]);
    }

    fn list_param2(ptr: String) {
        assert_eq!(ptr, "foo");
    }

    fn list_param3(ptr: Vec<String>) {
        assert_eq!(ptr.len(), 3);
        assert_eq!(ptr[0], "foo");
        assert_eq!(ptr[1], "bar");
        assert_eq!(ptr[2], "baz");
    }

    fn list_param4(ptr: Vec<Vec<String>>) {
        assert_eq!(ptr.len(), 2);
        assert_eq!(ptr[0][0], "foo");
        assert_eq!(ptr[0][1], "bar");
        assert_eq!(ptr[1][0], "baz");
    }

    fn list_result() -> Vec<u8> {
        vec![1, 2, 3, 4, 5]
    }

    fn list_result2() -> String {
        "hello!".to_string()
    }

    fn list_result3() -> Vec<String> {
        vec!["hello,".to_string(), "world!".to_string()]
    }

    fn string_roundtrip(x: String) -> String {
        x.clone()
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

    fn wasm_state_create() -> Handle<WasmState> {
        WasmState(100).into()
    }

    fn wasm_state_get_val(state: Handle<WasmState>) -> u32 {
        state.0
    }

    fn wasm_state2_create() -> Handle<WasmState2> {
        WasmState2(33).into()
    }

    fn wasm_state2_saw_close() -> bool {
        CLOSED.load(SeqCst) != 0
    }

    fn drop_wasm_state2(_state: WasmState2) {
        CLOSED.store(1, SeqCst);
    }

    fn two_wasm_states(
        _a: Handle<WasmState>,
        _b: Handle<WasmState2>,
    ) -> (Handle<WasmState>, Handle<WasmState2>) {
        (WasmState(101).into(), WasmState2(102).into())
    }

    fn wasm_state2_param_record(_a: WasmStateParamRecord) {}
    fn wasm_state2_param_tuple(_a: (Handle<WasmState2>,)) {}
    fn wasm_state2_param_option(_a: Option<Handle<WasmState2>>) {}
    fn wasm_state2_param_result(_a: Result<Handle<WasmState2>, u32>) {}
    fn wasm_state2_param_variant(_a: WasmStateParamVariant) {}
    fn wasm_state2_param_list(_a: Vec<Handle<WasmState2>>) {}

    fn wasm_state2_result_record() -> WasmStateResultRecord {
        WasmStateResultRecord {
            a: WasmState2(222).into(),
        }
    }
    fn wasm_state2_result_tuple() -> (Handle<WasmState2>,) {
        (WasmState2(333).into(),)
    }
    fn wasm_state2_result_option() -> Option<Handle<WasmState2>> {
        Some(WasmState2(444).into())
    }
    fn wasm_state2_result_result() -> Result<Handle<WasmState2>, u32> {
        Ok(WasmState2(555).into())
    }
    fn wasm_state2_result_variant() -> WasmStateResultVariant {
        WasmStateResultVariant::V0(Handle::new(WasmState2(666)))
    }
    fn wasm_state2_result_list() -> Vec<Handle<WasmState2>> {
        vec![WasmState2(777).into(), WasmState2(888).into()]
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

#[derive(Default)]
pub struct Markdown {
    buf: RefCell<String>,
}

impl wasm::Markdown for Markdown {
    fn create() -> Option<Handle<Markdown>> {
        Some(Markdown::default().into())
    }

    fn append(&self, input: String) {
        self.buf.borrow_mut().push_str(&input);
    }

    fn render(&self) -> String {
        self.buf.borrow().replace("red", "green")
    }
}
