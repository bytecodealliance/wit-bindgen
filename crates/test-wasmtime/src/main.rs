use std::cell::Cell;
use wasmtime::*;
use witx_bindgen_wasmtime::{BorrowChecker, GuestPtr, Table};

const CHECKED: &[u8] = include_bytes!(env!("CHECKED"));
const UNCHECKED: &[u8] = include_bytes!(env!("UNCHECKED"));

witx_bindgen_wasmtime::import!("tests/host.witx");

#[derive(Default)]
struct MyHost {
    scalar: Cell<u32>,
    borrow_checker: BorrowChecker,
    host_state_table: Table<SuchState>,
    host_state2_table: Table<()>,
    host_state2_closed: Cell<bool>,
}

struct SuchState(u32);

impl Host for MyHost {
    type HostState = SuchState;
    type HostState2 = ();

    fn host_state_table(&self) -> &Table<SuchState> {
        &self.host_state_table
    }

    fn host_state2_table(&self) -> &Table<()> {
        &self.host_state2_table
    }

    fn borrow_checker(&self) -> &BorrowChecker {
        &self.borrow_checker
    }

    fn roundtrip_u8(&self, val: u8) -> u8 {
        val
    }

    fn roundtrip_s8(&self, val: i8) -> i8 {
        val
    }

    fn roundtrip_u16(&self, val: u16) -> u16 {
        val
    }

    fn roundtrip_s16(&self, val: i16) -> i16 {
        val
    }

    fn roundtrip_u32(&self, val: u32) -> u32 {
        val
    }

    fn roundtrip_s32(&self, val: i32) -> i32 {
        val
    }

    fn roundtrip_u64(&self, val: u64) -> u64 {
        val
    }

    fn roundtrip_s64(&self, val: i64) -> i64 {
        val
    }

    fn roundtrip_usize(&self, val: u32) -> u32 {
        val
    }

    fn roundtrip_f32(&self, val: f32) -> f32 {
        val
    }

    fn roundtrip_f64(&self, val: f64) -> f64 {
        val
    }

    fn roundtrip_char(&self, val: char) -> char {
        val
    }

    fn multiple_results(&self) -> (u8, u16) {
        (4, 5)
    }

    fn set_scalar(&self, val: u32) {
        self.scalar.set(val);
    }

    fn get_scalar(&self) -> u32 {
        self.scalar.get()
    }

    fn swap_tuple(&self, a: (u8, u32)) -> (u32, u8) {
        (a.1, a.0)
    }

    fn roundtrip_flags1(&self, a: F1) -> F1 {
        drop(a.to_string());
        drop(format!("{:?}", a));
        drop(a & F1::all());
        a
    }

    fn roundtrip_flags2(&self, a: F2) -> F2 {
        a
    }

    fn roundtrip_record1(&self, a: R1) -> R1 {
        drop(format!("{:?}", a));
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

    fn legacy_params(
        &self,
        a: (u32, u32),
        _: R1,
        _: (u8, i8, u16, i16, u32, i32, u64, i64, f32, f64),
    ) {
        assert_eq!(a, (1, 2));
    }

    fn legacy_result(&self, succeed: bool) -> Result<LegacyResult, E1> {
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

    fn list_param(&self, ptr: GuestPtr<'_, [u8]>) {
        let list = ptr.borrow().unwrap();
        assert_eq!(*list, [1, 2, 3, 4]);
        assert!(ptr.borrow().is_ok());
        assert!(ptr.borrow_mut().is_err());
        drop(list);
        assert!(ptr.borrow().is_ok());
        assert!(ptr.borrow_mut().is_ok());
    }

    fn list_param2(&self, ptr: GuestPtr<'_, str>) {
        assert_eq!(&*ptr.borrow().unwrap(), "foo");
    }

    fn list_param3(&self, ptr: Vec<GuestPtr<'_, str>>) {
        assert_eq!(ptr.len(), 3);
        assert_eq!(&*ptr[0].borrow().unwrap(), "foo");
        assert_eq!(&*ptr[1].borrow().unwrap(), "bar");
        assert_eq!(&*ptr[2].borrow().unwrap(), "baz");
    }

    fn list_param4(&self, ptr: Vec<Vec<GuestPtr<'_, str>>>) {
        assert_eq!(ptr.len(), 2);
        assert_eq!(&*ptr[0][0].borrow().unwrap(), "foo");
        assert_eq!(&*ptr[0][1].borrow().unwrap(), "bar");
        assert_eq!(&*ptr[1][0].borrow().unwrap(), "baz");
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

    fn list_in_record1(&self, ty: ListInRecord1<'_>) {
        assert_eq!(&*ty.a.borrow().unwrap(), "list_in_record1");
    }

    fn list_in_record2(&self) -> ListInRecord2 {
        ListInRecord2 {
            a: "list_in_record2".to_string(),
        }
    }

    fn list_in_record3(&self, a: ListInRecord3Param<'_>) -> ListInRecord3Result {
        assert_eq!(&*a.a.borrow().unwrap(), "list_in_record3 input");
        ListInRecord3Result {
            a: "list_in_record3 output".to_string(),
        }
    }

    fn list_in_record4(&self, a: ListInAliasParam<'_>) -> ListInAliasResult {
        assert_eq!(&*a.a.borrow().unwrap(), "input4");
        ListInRecord4Result {
            a: "result4".to_string(),
        }
    }

    fn list_in_variant1(
        &self,
        a: ListInVariant11<'_>,
        b: ListInVariant12<'_>,
        c: ListInVariant13<'_>,
    ) {
        assert_eq!(&*a.unwrap().borrow().unwrap(), "foo");
        assert_eq!(&*b.unwrap_err().borrow().unwrap(), "bar");
        match c {
            ListInVariant13::V0(s) => assert_eq!(&*s.borrow().unwrap(), "baz"),
            ListInVariant13::V1(_) => panic!(),
        }
    }

    fn list_in_variant2(&self) -> Option<String> {
        Some("list_in_variant2".to_string())
    }

    fn list_in_variant3(&self, a: ListInVariant3Param<'_>) -> Option<String> {
        assert_eq!(&*a.unwrap().borrow().unwrap(), "input3");
        Some("output3".to_string())
    }

    fn errno_result(&self) -> Result<(), MyErrno> {
        MyErrno::A.to_string();
        format!("{:?}", MyErrno::A);
        fn assert_error<T: std::error::Error>() {}
        assert_error::<MyErrno>();
        Err(MyErrno::B)
    }

    fn list_typedefs(
        &self,
        a: ListTypedef<'_>,
        b: ListTypedef3Param<'_>,
    ) -> (ListTypedef2, ListTypedef3Result) {
        assert_eq!(&*a.borrow().unwrap(), "typedef1");
        assert_eq!(b.len(), 1);
        assert_eq!(&*b[0].borrow().unwrap(), "typedef2");
        (b"typedef3".to_vec(), vec!["typedef4".to_string()])
    }

    fn host_state_create(&self) -> SuchState {
        SuchState(100)
    }

    fn host_state_get(&self, state: &SuchState) -> u32 {
        state.0
    }

    fn host_state2_create(&self) {}

    fn host_state2_saw_close(&self) -> bool {
        self.host_state2_closed.get()
    }

    fn host_state2_close(&self, _state: ()) {
        self.host_state2_closed.set(true);
    }

    fn two_host_states(&self, _a: &SuchState, _b: &()) -> (SuchState, ()) {
        (SuchState(2), ())
    }
}

fn main() -> anyhow::Result<()> {
    // Create an engine with caching enabled to assist with iteration in this
    // project.
    let mut config = Config::new();
    config.cache_config_load_default()?;
    let engine = Engine::new(&config);

    run_test(&engine, CHECKED)?;
    run_test(&engine, UNCHECKED)?;

    Ok(())
}

fn run_test(engine: &Engine, wasm: &[u8]) -> anyhow::Result<()> {
    // Compile our wasm module ...
    let module = Module::new(&engine, wasm)?;

    // Create a linker with WASI functions ...
    let store = Store::new(&engine);
    let mut linker = Linker::new(&store);
    wasmtime_wasi::Wasi::new(
        &store,
        wasmtime_wasi::WasiCtxBuilder::new()
            .inherit_stdio()
            .build()?,
    )
    .add_to_linker(&mut linker)?;

    // Add our witx-defined functions to the linker
    add_host_to_linker(MyHost::default(), &mut linker)?;

    // And now we can run the whole test!
    let instance = linker.instantiate(&module)?;
    run(&instance, "run_host_tests")?;
    run_err(&instance, "invalid_bool", "invalid discriminant for `bool`")?;
    run_err(&instance, "invalid_u8", "out-of-bounds integer conversion")?;
    run_err(&instance, "invalid_s8", "out-of-bounds integer conversion")?;
    run_err(&instance, "invalid_u16", "out-of-bounds integer conversion")?;
    run_err(&instance, "invalid_s16", "out-of-bounds integer conversion")?;
    run_err(&instance, "invalid_char", "char value out of valid range")?;
    run_err(&instance, "invalid_e1", "invalid discriminant for `E1`")?;
    run_err(&instance, "invalid_handle", "invalid handle index")?;
    run_err(&instance, "invalid_handle_close", "invalid handle index")?;
    Ok(())
}

fn run(i: &Instance, name: &str) -> Result<()> {
    let run = i.get_func(name).unwrap();
    let run = run.get0::<()>()?;
    run()?;
    Ok(())
}

fn run_err(i: &Instance, name: &str, err: &str) -> Result<()> {
    match run(i, name) {
        Ok(()) => anyhow::bail!("export `{}` didn't trap", name),
        Err(e) if e.to_string().contains(err) => Ok(()),
        Err(e) => Err(e),
    }
}
