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
}

impl Host for MyHost {
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
