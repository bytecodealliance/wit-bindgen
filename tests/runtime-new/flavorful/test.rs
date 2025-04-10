include!(env!("BINDINGS"));

use exports::test::flavorful::to_test::*;

struct Component;

export!(Component);

impl Guest for Component {
    fn f_list_in_record1(ty: ListInRecord1) {
        assert_eq!(ty.a, "list_in_record1");
    }

    fn f_list_in_record2() -> ListInRecord2 {
        ListInRecord2 {
            a: "list_in_record2".to_string(),
        }
    }

    fn f_list_in_record3(a: ListInRecord3) -> ListInRecord3 {
        assert_eq!(a.a, "list_in_record3 input");
        ListInRecord3 {
            a: "list_in_record3 output".to_string(),
        }
    }

    fn f_list_in_record4(a: ListInAlias) -> ListInAlias {
        assert_eq!(a.a, "input4");
        ListInRecord4 {
            a: "result4".to_string(),
        }
    }

    fn f_list_in_variant1(a: ListInVariant1V1, b: ListInVariant1V2) {
        assert_eq!(a.unwrap(), "foo");
        assert_eq!(b.unwrap_err(), "bar");
    }

    fn f_list_in_variant2() -> Option<String> {
        Some("list_in_variant2".to_string())
    }

    fn f_list_in_variant3(a: ListInVariant3) -> Option<String> {
        assert_eq!(a.unwrap(), "input3");
        Some("output3".to_string())
    }

    fn errno_result() -> Result<(), MyErrno> {
        static mut FIRST: bool = true;
        MyErrno::A.to_string();
        _ = format!("{:?}", MyErrno::A);
        fn assert_error<T: std::error::Error>() {}
        assert_error::<MyErrno>();

        unsafe {
            if FIRST {
                FIRST = false;
                Err(MyErrno::B)
            } else {
                Ok(())
            }
        }
    }

    fn list_typedefs(a: ListTypedef, b: ListTypedef3) -> (ListTypedef2, ListTypedef3) {
        assert_eq!(a, "typedef1");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0], "typedef2");
        (b"typedef3".to_vec(), vec!["typedef4".to_string()])
    }

    fn list_of_variants(
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
