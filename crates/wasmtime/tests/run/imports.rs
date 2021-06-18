use witx_bindgen_wasmtime::imports::{PullBuffer, PushBuffer};

witx_bindgen_wasmtime::import!("tests/host.witx");
use host::*;

pub(crate) use host::{add_host_to_linker, HostTables};

#[derive(Default)]
pub struct MyHost {
    scalar: u32,
    host_state2_closed: bool,
}

pub struct SuchState(u32);

// TODO: implement propagation of errors instead of `unwrap()` everywhere

impl Host for MyHost {
    type HostState = SuchState;
    type HostState2 = ();

    fn roundtrip_u8(&mut self, val: u8) -> u8 {
        val
    }

    fn roundtrip_s8(&mut self, val: i8) -> i8 {
        val
    }

    fn roundtrip_u16(&mut self, val: u16) -> u16 {
        val
    }

    fn roundtrip_s16(&mut self, val: i16) -> i16 {
        val
    }

    fn roundtrip_u32(&mut self, val: u32) -> u32 {
        val
    }

    fn roundtrip_s32(&mut self, val: i32) -> i32 {
        val
    }

    fn roundtrip_u64(&mut self, val: u64) -> u64 {
        val
    }

    fn roundtrip_s64(&mut self, val: i64) -> i64 {
        val
    }

    fn roundtrip_usize(&mut self, val: u32) -> u32 {
        val
    }

    fn roundtrip_f32(&mut self, val: f32) -> f32 {
        val
    }

    fn roundtrip_f64(&mut self, val: f64) -> f64 {
        val
    }

    fn roundtrip_char(&mut self, val: char) -> char {
        val
    }

    fn multiple_results(&mut self) -> (u8, u16) {
        (4, 5)
    }

    fn set_scalar(&mut self, val: u32) {
        self.scalar = val;
    }

    fn get_scalar(&mut self) -> u32 {
        self.scalar
    }

    fn swap_tuple(&mut self, a: (u8, u32)) -> (u32, u8) {
        (a.1, a.0)
    }

    fn roundtrip_flags1(&mut self, a: F1) -> F1 {
        drop(a.to_string());
        drop(format!("{:?}", a));
        drop(a & F1::all());
        a
    }

    fn roundtrip_flags2(&mut self, a: F2) -> F2 {
        a
    }

    fn roundtrip_flags3(
        &mut self,
        a: Flag8,
        b: Flag16,
        c: Flag32,
        d: Flag64,
    ) -> (Flag8, Flag16, Flag32, Flag64) {
        (a, b, c, d)
    }

    fn legacy_flags1(&mut self, a: Flag8) -> Flag8 {
        a
    }

    fn legacy_flags2(&mut self, a: Flag16) -> Flag16 {
        a
    }

    fn legacy_flags3(&mut self, a: Flag32) -> Flag32 {
        a
    }

    fn legacy_flags4(&mut self, a: Flag64) -> Flag64 {
        a
    }

    fn roundtrip_record1(&mut self, a: R1) -> R1 {
        drop(format!("{:?}", a));
        a
    }

    fn tuple0(&mut self, _: ()) {}

    fn tuple1(&mut self, a: (u8,)) -> (u8,) {
        (a.0,)
    }

    fn roundtrip_option(&mut self, a: Option<f32>) -> Option<u8> {
        a.map(|x| x as u8)
    }

    fn roundtrip_result(&mut self, a: Result<u32, f32>) -> Result<f64, u8> {
        match a {
            Ok(a) => Ok(a.into()),
            Err(b) => Err(b as u8),
        }
    }

    fn roundtrip_enum(&mut self, a: E1) -> E1 {
        assert_eq!(a, a);
        a
    }

    fn invert_bool(&mut self, a: bool) -> bool {
        !a
    }

    fn variant_casts(&mut self, a: Casts) -> Casts {
        a
    }

    fn variant_zeros(&mut self, a: Zeros) -> Zeros {
        a
    }

    fn variant_typedefs(&mut self, _: Option<u32>, _: bool, _: Result<u32, ()>) {}

    fn variant_enums(
        &mut self,
        a: bool,
        b: Result<(), ()>,
        c: MyErrno,
    ) -> (bool, Result<(), ()>, MyErrno) {
        assert_eq!(a, true);
        assert_eq!(b, Ok(()));
        assert_eq!(c, MyErrno::Success);
        (false, Err(()), MyErrno::A)
    }

    fn legacy_params(
        &mut self,
        a: (u32, u32),
        _: R1,
        _: (u8, i8, u16, i16, u32, i32, u64, i64, f32, f64),
    ) {
        assert_eq!(a, (1, 2));
    }

    fn legacy_result(&mut self, succeed: bool) -> Result<LegacyResult, E1> {
        if succeed {
            Ok((
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                9.,
                10.,
                R1 {
                    a: 0,
                    b: F1::empty(),
                },
            ))
        } else {
            Err(E1::B)
        }
    }

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

    fn buffer_u32(&mut self, in_: &[u32], out: &mut [u32]) -> u32 {
        assert_eq!(in_, [0]);
        assert_eq!(out.len(), 10);
        out[0] = 1;
        out[1] = 2;
        out[2] = 3;
        3
    }

    fn buffer_bool(&mut self, in_: PullBuffer<'_, bool>, mut out: PushBuffer<'_, bool>) -> u32 {
        assert!(in_.len() < out.capacity());
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
}
