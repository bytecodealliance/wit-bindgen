#[cfg(not(feature = "unchecked"))]
witx_bindgen_rust::export!("tests/wasm.witx");

#[cfg(feature = "unchecked")]
witx_bindgen_rust::export!({ paths: ["tests/wasm.witx"], unchecked });

use wasm::*;

use std::sync::atomic::{AtomicU32, Ordering::SeqCst};
use witx_bindgen_rust::exports::{InBuffer, InBufferRaw, OutBuffer, OutBufferRaw};

struct MyWasm {
    scalar: AtomicU32,
    wasm_state2_closed: AtomicU32,
}

fn wasm() -> &'static impl Wasm {
    static ME: MyWasm = MyWasm {
        scalar: AtomicU32::new(0),
        wasm_state2_closed: AtomicU32::new(0),
    };
    &ME
}

struct MyType(u32);

struct MyType2(u32);

impl Wasm for MyWasm {
    type WasmState = MyType;
    type WasmState2 = MyType2;

    fn allocated_bytes(&self) -> u32 {
        crate::allocator::get() as u32
    }

    fn run_import_tests(&self) {
        crate::imports::run();
    }

    fn roundtrip_u8(&self, a: u8) -> u8 {
        a
    }

    fn roundtrip_s8(&self, a: i8) -> i8 {
        a
    }

    fn roundtrip_u16(&self, a: u16) -> u16 {
        a
    }

    fn roundtrip_s16(&self, a: i16) -> i16 {
        a
    }

    fn roundtrip_u32(&self, a: u32) -> u32 {
        a
    }

    fn roundtrip_s32(&self, a: i32) -> i32 {
        a
    }

    fn roundtrip_u64(&self, a: u64) -> u64 {
        a
    }

    fn roundtrip_s64(&self, a: i64) -> i64 {
        a
    }

    fn roundtrip_f32(&self, a: f32) -> f32 {
        a
    }

    fn roundtrip_f64(&self, a: f64) -> f64 {
        a
    }

    fn roundtrip_char(&self, a: char) -> char {
        a
    }

    fn multiple_results(&self) -> (u8, u16) {
        (100, 200)
    }

    fn set_scalar(&self, val: u32) {
        self.scalar.store(val, SeqCst)
    }

    fn get_scalar(&self) -> u32 {
        self.scalar.load(SeqCst)
    }

    fn swap_tuple(&self, a: (u8, u32)) -> (u32, u8) {
        (a.1, a.0)
    }

    fn roundtrip_flags1(&self, a: F1) -> F1 {
        a
    }

    fn roundtrip_flags2(&self, a: F2) -> F2 {
        a
    }

    fn roundtrip_record1(&self, a: R1) -> R1 {
        a
    }

    fn tuple0(&self, _: ()) {}

    fn tuple1(&self, a: (u8,)) -> (u8,) {
        (a.0,)
    }

    fn roundtrip_option(&self, a: Option<f32>) -> Option<u8> {
        a.map(|x| x as u8)
    }

    fn roundtrip_result(&self, a: Result<u32, f32>) -> Result<f64, u8> {
        match a {
            Ok(a) => Ok(a.into()),
            Err(b) => Err(b as u8),
        }
    }

    fn roundtrip_enum(&self, a: E1) -> E1 {
        assert_eq!(a, a);
        a
    }

    fn invert_bool(&self, a: bool) -> bool {
        !a
    }

    fn variant_casts(&self, a: Casts) -> Casts {
        a
    }

    fn variant_zeros(&self, a: Zeros) -> Zeros {
        a
    }

    fn variant_typedefs(&self, _: Option<u32>, _: bool, _: Result<u32, ()>) {}

    fn list_param(&self, list: Vec<u8>) {
        assert_eq!(list, [1, 2, 3, 4]);
    }

    fn list_param2(&self, ptr: String) {
        assert_eq!(ptr, "foo");
    }

    fn list_param3(&self, ptr: Vec<String>) {
        assert_eq!(ptr.len(), 3);
        assert_eq!(ptr[0], "foo");
        assert_eq!(ptr[1], "bar");
        assert_eq!(ptr[2], "baz");
    }

    fn list_param4(&self, ptr: Vec<Vec<String>>) {
        assert_eq!(ptr.len(), 2);
        assert_eq!(ptr[0][0], "foo");
        assert_eq!(ptr[0][1], "bar");
        assert_eq!(ptr[1][0], "baz");
    }

    fn list_result(&self) -> Vec<u8> {
        vec![1, 2, 3, 4, 5]
    }

    fn list_result2(&self) -> String {
        "hello!".to_string()
    }

    fn list_result3(&self) -> Vec<String> {
        vec!["hello,".to_string(), "world!".to_string()]
    }

    fn list_in_record1(&self, ty: ListInRecord1) {
        assert_eq!(ty.a, "list_in_record1");
    }

    fn list_in_record2(&self) -> ListInRecord2 {
        ListInRecord2 {
            a: "list_in_record2".to_string(),
        }
    }

    fn list_in_record3(&self, a: ListInRecord3) -> ListInRecord3 {
        assert_eq!(a.a, "list_in_record3 input");
        ListInRecord3 {
            a: "list_in_record3 output".to_string(),
        }
    }

    fn list_in_record4(&self, a: ListInAlias) -> ListInAlias {
        assert_eq!(a.a, "input4");
        ListInRecord4 {
            a: "result4".to_string(),
        }
    }

    fn list_in_variant1(&self, a: ListInVariant11, b: ListInVariant12, c: ListInVariant13) {
        assert_eq!(a.unwrap(), "foo");
        assert_eq!(b.unwrap_err(), "bar");
        match c {
            ListInVariant13::V0(s) => assert_eq!(s, "baz"),
            ListInVariant13::V1(_) => panic!(),
        }
    }

    fn list_in_variant2(&self) -> Option<String> {
        Some("list_in_variant2".to_string())
    }

    fn list_in_variant3(&self, a: ListInVariant3) -> Option<String> {
        assert_eq!(a.unwrap(), "input3");
        Some("output3".to_string())
    }

    fn errno_result(&self) -> Result<(), MyErrno> {
        MyErrno::A.to_string();
        format!("{:?}", MyErrno::A);
        fn assert_error<T: std::error::Error>() {}
        assert_error::<MyErrno>();
        Err(MyErrno::B)
    }

    fn list_typedefs(&self, a: ListTypedef, b: ListTypedef3) -> (ListTypedef2, ListTypedef3) {
        assert_eq!(a, "typedef1");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0], "typedef2");
        (b"typedef3".to_vec(), vec!["typedef4".to_string()])
    }

    fn wasm_state_create(&self) -> MyType {
        MyType(100)
    }

    fn wasm_state_get(&self, state: &MyType) -> u32 {
        state.0
    }

    fn wasm_state2_create(&self) -> MyType2 {
        MyType2(33)
    }

    fn wasm_state2_saw_close(&self) -> bool {
        self.wasm_state2_closed.load(SeqCst) != 0
    }

    fn wasm_state2_close(&self, _state: MyType2) {
        self.wasm_state2_closed.store(1, SeqCst);
    }

    fn two_wasm_states(&self, _a: &MyType, _b: &MyType2) -> (MyType, MyType2) {
        (MyType(101), MyType2(102))
    }

    fn wasm_state2_param_record(&self, _a: WasmStateParamRecord<'_, Self>) {}
    fn wasm_state2_param_tuple(&self, _a: (&'_ MyType2,)) {}
    fn wasm_state2_param_option(&self, _a: Option<&'_ MyType2>) {}
    fn wasm_state2_param_result(&self, _a: Result<&'_ MyType2, u32>) {}
    fn wasm_state2_param_variant(&self, _a: WasmStateParamVariant<'_, Self>) {}
    fn wasm_state2_param_list(&self, _a: Vec<&MyType2>) {}

    fn wasm_state2_result_record(&self) -> WasmStateResultRecord<Self> {
        WasmStateResultRecord { a: MyType2(222) }
    }
    fn wasm_state2_result_tuple(&self) -> (MyType2,) {
        (MyType2(333),)
    }
    fn wasm_state2_result_option(&self) -> Option<MyType2> {
        Some(MyType2(444))
    }
    fn wasm_state2_result_result(&self) -> Result<MyType2, u32> {
        Ok(MyType2(555))
    }
    fn wasm_state2_result_variant(&self) -> WasmStateResultVariant<Self> {
        WasmStateResultVariant::V0(MyType2(666))
    }
    fn wasm_state2_result_list(&self) -> Vec<MyType2> {
        vec![MyType2(777), MyType2(888)]
    }

    fn buffer_u8(&self, in_: InBufferRaw<'_, u8>, out: OutBufferRaw<'_, u8>) -> u32 {
        assert_eq!(in_.len(), 1);
        let mut input = [0];
        in_.copy(&mut input);
        assert_eq!(input, [0]);

        assert_eq!(out.capacity(), 10);
        out.write(&[1, 2, 3]);
        3
    }

    fn buffer_u32(&self, in_: InBufferRaw<'_, u32>, out: OutBufferRaw<'_, u32>) -> u32 {
        assert_eq!(in_.len(), 1);
        let mut input = [0];
        in_.copy(&mut input);
        assert_eq!(input, [0]);

        assert_eq!(out.capacity(), 10);
        out.write(&[1, 2, 3]);
        3
    }

    fn buffer_bool(&self, in_: InBuffer<'_, bool>, out: OutBuffer<'_, bool>) -> u32 {
        assert!(in_.len() <= out.capacity());
        let len = in_.len();
        let mut storage = vec![0; in_.len() * in_.element_size()];
        let items = in_.iter(&mut storage).map(|b| !b).collect::<Vec<_>>();
        out.write(&mut storage, items.into_iter());
        len as u32
    }

    fn buffer_string(&self, in_: InBuffer<'_, String>, out: OutBuffer<'_, String>) -> u32 {
        assert!(in_.len() <= out.capacity());
        let len = in_.len();
        let mut storage = vec![0; in_.len() * in_.element_size()];
        let items = in_
            .iter(&mut storage)
            .map(|s| s.to_uppercase())
            .collect::<Vec<_>>();
        out.write(&mut storage, items.into_iter());
        len as u32
    }

    fn buffer_list_bool(&self, in_: InBuffer<'_, Vec<bool>>, out: OutBuffer<'_, Vec<bool>>) -> u32 {
        assert!(in_.len() <= out.capacity());
        let len = in_.len();
        let mut storage = vec![0; in_.len() * in_.element_size()];
        let items = in_
            .iter(&mut storage)
            .map(|s| s.into_iter().map(|b| !b).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        out.write(&mut storage, items.into_iter());
        len as u32
    }

    // fn buffer_buffer_bool(&self, in_: InBuffer<'_, InBuffer<'_, bool>>) {
    //     assert_eq!(in_.len(), 1);
    //     let mut storage = vec![0; in_.len() * in_.element_size()];
    //     let buf = in_.iter(&mut storage).next().unwrap();
    //     assert_eq!(buf.len(), 5);
    //     let mut storage2 = vec![0; buf.len() * buf.element_size()];
    //     assert_eq!(
    //         buf.iter(&mut storage2).collect::<Vec<bool>>(),
    //         [true, false, true, true, false]
    //     );
    // }

    fn buffer_mutable1(&self, a: Vec<InBuffer<'_, bool>>) {
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].len(), 5);
        let mut storage = vec![0; a[0].len() * a[0].element_size()];
        assert_eq!(
            a[0].iter(&mut storage).collect::<Vec<_>>(),
            [true, false, true, true, false]
        );
    }

    fn buffer_mutable2(&self, a: Vec<OutBufferRaw<'_, u8>>) -> u32 {
        assert_eq!(a.len(), 1);
        assert!(a[0].capacity() > 4);
        a[0].write(&[1, 2, 3, 4]);
        return 4;
    }

    fn buffer_mutable3(&self, a: Vec<OutBuffer<'_, bool>>) -> u32 {
        assert_eq!(a.len(), 1);
        assert!(a[0].capacity() > 3);
        let mut storage = [0; 200];
        a[0].write(&mut storage, [false, true, false].iter().copied());
        return 3;
    }

    fn buffer_in_record(&self, _: BufferInRecord<'_>) {}
    fn buffer_typedef(
        &self,
        _: ParamInBufferU8<'_>,
        _: ParamOutBufferU8<'_>,
        _: ParamInBufferBool<'_>,
        _: ParamOutBufferBool<'_>,
    ) {
    }
}
