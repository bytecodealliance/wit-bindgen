witx_bindgen_wasmtime::import!("tests/host.witx");

use host::*;
pub(crate) use host::{add_host_to_linker, HostTables};
use std::cell::RefCell;
use witx_bindgen_wasmtime::{
    imports::{PullBuffer, PushBuffer},
    Le,
};

#[derive(Default)]
pub struct MyHost {
    scalar: u32,
    host_state2_closed: bool,
}

#[derive(Debug)]
pub struct SuchState(u32);

#[derive(Default, Debug)]
pub struct Markdown {
    buf: RefCell<String>,
}

// TODO: implement propagation of errors instead of `unwrap()` everywhere

impl Host for MyHost {
    type HostState = SuchState;
    type HostState2 = ();
    type Markdown2 = Markdown;

    fn list_param(&mut self, list: &[u8]) {
        assert_eq!(list, [1, 2, 3, 4]);
    }

    fn list_param2(&mut self, ptr: &str) {
        assert_eq!(ptr, "foo");
    }

    fn list_param3(&mut self, ptr: Vec<&str>) {
        assert_eq!(ptr.len(), 3);
        assert_eq!(ptr[0], "foo");
        assert_eq!(ptr[1], "bar");
        assert_eq!(ptr[2], "baz");
    }

    fn list_param4(&mut self, ptr: Vec<Vec<&str>>) {
        assert_eq!(ptr.len(), 2);
        assert_eq!(ptr[0][0], "foo");
        assert_eq!(ptr[0][1], "bar");
        assert_eq!(ptr[1][0], "baz");
    }

    fn list_result(&mut self) -> Vec<u8> {
        vec![1, 2, 3, 4, 5]
    }

    fn list_result2(&mut self) -> String {
        "hello!".to_string()
    }

    fn list_result3(&mut self) -> Vec<String> {
        vec!["hello,".to_string(), "world!".to_string()]
    }

    fn string_roundtrip(&mut self, s: &str) -> String {
        s.to_string()
    }

    fn list_in_record1(&mut self, ty: ListInRecord1<'_>) {
        assert_eq!(ty.a, "list_in_record1");
    }

    fn list_in_record2(&mut self) -> ListInRecord2 {
        ListInRecord2 {
            a: "list_in_record2".to_string(),
        }
    }

    fn list_in_record3(&mut self, a: ListInRecord3Param<'_>) -> ListInRecord3Result {
        assert_eq!(a.a, "list_in_record3 input");
        ListInRecord3Result {
            a: "list_in_record3 output".to_string(),
        }
    }

    fn list_in_record4(&mut self, a: ListInAliasParam<'_>) -> ListInAliasResult {
        assert_eq!(a.a, "input4");
        ListInRecord4Result {
            a: "result4".to_string(),
        }
    }

    fn list_in_variant1(
        &mut self,
        a: ListInVariant11<'_>,
        b: ListInVariant12<'_>,
        c: ListInVariant13<'_>,
    ) {
        assert_eq!(a.unwrap(), "foo");
        assert_eq!(b.unwrap_err(), "bar");
        match c {
            ListInVariant13::V0(s) => assert_eq!(s, "baz"),
            ListInVariant13::V1(_) => panic!(),
        }
    }

    fn list_in_variant2(&mut self) -> Option<String> {
        Some("list_in_variant2".to_string())
    }

    fn list_in_variant3(&mut self, a: ListInVariant3Param<'_>) -> Option<String> {
        assert_eq!(a.unwrap(), "input3");
        Some("output3".to_string())
    }

    fn errno_result(&mut self) -> Result<(), MyErrno> {
        MyErrno::A.to_string();
        format!("{:?}", MyErrno::A);
        fn assert_error<T: std::error::Error>() {}
        assert_error::<MyErrno>();
        Err(MyErrno::B)
    }

    fn list_typedefs(
        &mut self,
        a: ListTypedef<'_>,
        b: ListTypedef3Param<'_>,
    ) -> (ListTypedef2, ListTypedef3Result) {
        assert_eq!(a, "typedef1");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0], "typedef2");
        (b"typedef3".to_vec(), vec!["typedef4".to_string()])
    }

    fn host_state_create(&mut self) -> SuchState {
        SuchState(100)
    }

    fn host_state_get(&mut self, state: &SuchState) -> u32 {
        state.0
    }

    fn host_state2_create(&mut self) {}

    fn host_state2_saw_close(&mut self) -> bool {
        self.host_state2_closed
    }

    fn drop_host_state2(&mut self, _state: ()) {
        self.host_state2_closed = true;
    }

    fn two_host_states(&mut self, _a: &SuchState, _b: &()) -> (SuchState, ()) {
        (SuchState(2), ())
    }

    fn host_state2_param_record(&mut self, _a: HostStateParamRecord<'_, Self>) {}
    fn host_state2_param_tuple(&mut self, _a: (&'_ (),)) {}
    fn host_state2_param_option(&mut self, _a: Option<&'_ ()>) {}
    fn host_state2_param_result(&mut self, _a: Result<&'_ (), u32>) {}
    fn host_state2_param_variant(&mut self, _a: HostStateParamVariant<'_, Self>) {}
    fn host_state2_param_list(&mut self, _a: Vec<&()>) {}

    fn host_state2_result_record(&mut self) -> HostStateResultRecord<Self> {
        HostStateResultRecord { a: () }
    }
    fn host_state2_result_tuple(&mut self) -> ((),) {
        ((),)
    }
    fn host_state2_result_option(&mut self) -> Option<()> {
        Some(())
    }
    fn host_state2_result_result(&mut self) -> Result<(), u32> {
        Ok(())
    }
    fn host_state2_result_variant(&mut self) -> HostStateResultVariant<Self> {
        HostStateResultVariant::V0(())
    }
    fn host_state2_result_list(&mut self) -> Vec<()> {
        vec![(), ()]
    }

    fn buffer_u8(&mut self, in_: &[u8], out: &mut [u8]) -> u32 {
        assert_eq!(in_, [0]);
        assert_eq!(out.len(), 10);
        out[0] = 1;
        out[1] = 2;
        out[2] = 3;
        3
    }

    fn buffer_u32(&mut self, in_: &[Le<u32>], out: &mut [Le<u32>]) -> u32 {
        assert_eq!(in_.len(), 1);
        assert_eq!(in_[0].get(), 0);
        assert_eq!(out.len(), 10);
        out[0].set(1);
        out[1].set(2);
        out[2].set(3);
        3
    }

    fn buffer_bool(&mut self, in_: PullBuffer<'_, bool>, mut out: PushBuffer<'_, bool>) -> u32 {
        assert!(in_.len() <= out.capacity());
        let len = in_.len();
        for item in in_.iter() {
            let item = item.unwrap();
            out.write(Some(!item)).unwrap();
        }
        len as u32
    }

    // fn buffer_string(
    //     &mut self,
    //     in_: PullBuffer<'_, GuestPtr<'_, str>>,
    //     mut out: PushBuffer<'_, String>,
    // ) -> u32 {
    //     assert!(in_.len() < out.capacity());
    //     let len = in_.len();
    //     for item in in_.iter().unwrap() {
    //         let item = item.unwrap();
    //         let s = item.borrow().unwrap();
    //         out.write(Some(s.to_uppercase())).unwrap();
    //     }
    //     len as u32
    // }

    // fn buffer_list_bool(
    //     &mut self,
    //     in_: PullBuffer<'_, Vec<bool>>,
    //     mut out: PushBuffer<'_, Vec<bool>>,
    // ) -> u32 {
    //     assert!(in_.len() < out.capacity());
    //     let len = in_.len();
    //     for item in in_.iter().unwrap() {
    //         let item = item.unwrap();
    //         out.write(Some(item.into_iter().map(|b| !b).collect()))
    //             .unwrap();
    //     }
    //     len as u32
    // }

    // fn buffer_buffer_bool(&mut self, in_: PullBuffer<'_, PullBuffer<'_, bool>>) {
    //     assert_eq!(in_.len(), 1);
    //     let buf = in_.iter().unwrap().next().unwrap().unwrap();
    //     assert_eq!(buf.len(), 5);
    //     assert_eq!(
    //         buf.iter()
    //             .unwrap()
    //             .collect::<Result<Vec<bool>, _>>()
    //             .unwrap(),
    //         [true, false, true, true, false]
    //     );
    // }

    fn buffer_mutable1(&mut self, a: Vec<PullBuffer<'_, bool>>) {
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].len(), 5);
        assert_eq!(
            a[0].iter().collect::<Result<Vec<_>, _>>().unwrap(),
            [true, false, true, true, false]
        );
    }

    fn buffer_mutable2(&mut self, mut a: Vec<&mut [u8]>) -> u32 {
        assert_eq!(a.len(), 1);
        assert!(a[0].len() > 4);
        a[0][..4].copy_from_slice(&[1, 2, 3, 4]);
        return 4;
    }

    fn buffer_mutable3(&mut self, mut a: Vec<PushBuffer<'_, bool>>) -> u32 {
        assert_eq!(a.len(), 1);
        assert!(a[0].capacity() > 3);
        a[0].write([false, true, false].iter().copied()).unwrap();
        return 3;
    }

    fn buffer_in_record(&mut self, _: BufferInRecord<'_>) {}
    fn buffer_typedef(
        &mut self,
        _: ParamInBufferU8<'_>,
        _: ParamOutBufferU8<'_>,
        _: ParamInBufferBool<'_>,
        _: ParamOutBufferBool<'_>,
    ) {
    }

    fn list_of_variants(
        &mut self,
        bools: Vec<bool>,
        results: Vec<Result<(), ()>>,
        enums: Vec<MyErrno>,
    ) -> (Vec<bool>, Vec<Result<(), ()>>, Vec<MyErrno>) {
        assert_eq!(bools, [true, false]);
        assert_eq!(results, [Ok(()), Err(())]);
        assert_eq!(enums, [MyErrno::Success, MyErrno::A]);
        (
            vec![false, true],
            vec![Err(()), Ok(())],
            vec![MyErrno::A, MyErrno::B],
        )
    }

    fn unaligned_roundtrip1(
        &mut self,
        u16s: &[Le<u16>],
        u32s: &[Le<u32>],
        u64s: &[Le<u64>],
        flag32s: Vec<Flag32>,
        flag64s: Vec<Flag64>,
    ) {
        assert_eq!(u16s, [1]);
        assert_eq!(u32s, [2]);
        assert_eq!(u64s, [3]);
        assert_eq!(flag32s, [Flag32::B8]);
        assert_eq!(flag64s, [Flag64::B9]);
    }

    fn unaligned_roundtrip2(
        &mut self,
        records: &[Le<UnalignedRecord>],
        f32s: &[Le<f32>],
        f64s: &[Le<f64>],
        strings: Vec<&str>,
        lists: Vec<&[u8]>,
    ) {
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].get().a, 10);
        assert_eq!(records[0].get().b, 11);
        assert_eq!(f32s, [100.0]);
        assert_eq!(f64s, [101.0]);
        assert_eq!(strings, ["foo"]);
        assert_eq!(lists, [&[102][..]]);
    }

    fn markdown2_create(&mut self) -> Markdown {
        Markdown::default()
    }

    fn markdown2_append(&mut self, md: &Markdown, buf: &str) {
        md.buf.borrow_mut().push_str(buf);
    }

    fn markdown2_render(&mut self, md: &Markdown) -> String {
        md.buf.borrow().replace("red", "green")
    }

    fn list_minmax8(&mut self, u: &[u8], s: &[i8]) -> (Vec<u8>, Vec<i8>) {
        assert_eq!(u, [u8::MIN, u8::MAX]);
        assert_eq!(s, [i8::MIN, i8::MAX]);
        (u.to_vec(), s.to_vec())
    }

    fn list_minmax16(&mut self, u: &[Le<u16>], s: &[Le<i16>]) -> (Vec<u16>, Vec<i16>) {
        assert_eq!(u, [u16::MIN, u16::MAX]);
        assert_eq!(s, [i16::MIN, i16::MAX]);
        (
            u.iter().map(|e| e.get()).collect(),
            s.iter().map(|e| e.get()).collect(),
        )
    }

    fn list_minmax32(&mut self, u: &[Le<u32>], s: &[Le<i32>]) -> (Vec<u32>, Vec<i32>) {
        assert_eq!(u, [u32::MIN, u32::MAX]);
        assert_eq!(s, [i32::MIN, i32::MAX]);
        (
            u.iter().map(|e| e.get()).collect(),
            s.iter().map(|e| e.get()).collect(),
        )
    }

    fn list_minmax64(&mut self, u: &[Le<u64>], s: &[Le<i64>]) -> (Vec<u64>, Vec<i64>) {
        assert_eq!(u, [u64::MIN, u64::MAX]);
        assert_eq!(s, [i64::MIN, i64::MAX]);
        (
            u.iter().map(|e| e.get()).collect(),
            s.iter().map(|e| e.get()).collect(),
        )
    }

    fn list_minmax_float(&mut self, u: &[Le<f32>], s: &[Le<f64>]) -> (Vec<f32>, Vec<f64>) {
        assert_eq!(u, [f32::MIN, f32::MAX, f32::NEG_INFINITY, f32::INFINITY]);
        assert_eq!(s, [f64::MIN, f64::MAX, f64::NEG_INFINITY, f64::INFINITY]);
        (
            u.iter().map(|e| e.get()).collect(),
            s.iter().map(|e| e.get()).collect(),
        )
    }
}
