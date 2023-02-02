use anyhow::Result;
use exports::*;
use wasmtime::Store;

wasmtime::component::bindgen!("world" in "tests/runtime/flavorful");

#[derive(Default)]
pub struct MyImports {
    errored: bool,
}

impl imports::Imports for MyImports {
    fn f_list_in_record1(&mut self, ty: imports::ListInRecord1Result) -> Result<()> {
        assert_eq!(ty.a, "list_in_record1");
        Ok(())
    }

    fn f_list_in_record2(&mut self) -> Result<imports::ListInRecord2> {
        Ok(imports::ListInRecord2 {
            a: "list_in_record2".to_string(),
        })
    }

    fn f_list_in_record3(
        &mut self,
        a: imports::ListInRecord3Result,
    ) -> Result<imports::ListInRecord3Result> {
        assert_eq!(a.a, "list_in_record3 input");
        Ok(imports::ListInRecord3Result {
            a: "list_in_record3 output".to_string(),
        })
    }

    fn f_list_in_record4(
        &mut self,
        a: imports::ListInAliasResult,
    ) -> Result<imports::ListInAliasResult> {
        assert_eq!(a.a, "input4");
        Ok(imports::ListInRecord4Result {
            a: "result4".to_string(),
        })
    }

    fn f_list_in_variant1(
        &mut self,
        a: imports::ListInVariant1V1Result,
        b: imports::ListInVariant1V2Result,
        c: imports::ListInVariant1V3Result,
    ) -> Result<()> {
        assert_eq!(a.unwrap(), "foo");
        assert_eq!(b.unwrap_err(), "bar");
        match c {
            imports::ListInVariant1V3Result::String(s) => assert_eq!(s, "baz"),
            imports::ListInVariant1V3Result::F32(_) => panic!(),
        }
        Ok(())
    }

    fn f_list_in_variant2(&mut self) -> Result<Option<String>> {
        Ok(Some("list_in_variant2".to_string()))
    }

    fn f_list_in_variant3(&mut self, a: imports::ListInVariant3Result) -> Result<Option<String>> {
        assert_eq!(a.unwrap(), "input3");
        Ok(Some("output3".to_string()))
    }

    fn errno_result(&mut self) -> Result<Result<(), imports::MyErrno>> {
        if self.errored {
            return Ok(Ok(()));
        }
        imports::MyErrno::A.to_string();
        format!("{:?}", imports::MyErrno::A);
        fn assert_error<T: std::error::Error>() {}
        assert_error::<imports::MyErrno>();
        self.errored = true;
        Ok(Err(imports::MyErrno::B))
    }

    fn list_typedefs(
        &mut self,
        a: imports::ListTypedefResult,
        b: imports::ListTypedef3Result,
    ) -> Result<(imports::ListTypedef2, imports::ListTypedef3Result)> {
        assert_eq!(a, "typedef1");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0], "typedef2");
        Ok((b"typedef3".to_vec(), vec!["typedef4".to_string()]))
    }

    fn list_of_variants(
        &mut self,
        bools: Vec<bool>,
        results: Vec<Result<(), ()>>,
        enums: Vec<imports::MyErrno>,
    ) -> Result<(Vec<bool>, Vec<Result<(), ()>>, Vec<imports::MyErrno>)> {
        assert_eq!(bools, [true, false]);
        assert_eq!(results, [Ok(()), Err(())]);
        assert_eq!(enums, [imports::MyErrno::Success, imports::MyErrno::A]);
        Ok((
            vec![false, true],
            vec![Err(()), Ok(())],
            vec![imports::MyErrno::A, imports::MyErrno::B],
        ))
    }
}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "flavorful",
        |linker| Flavorful::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| Flavorful::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(exports: Flavorful, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    exports.call_test_imports(&mut *store)?;
    let exports = exports.exports();

    exports.call_f_list_in_record1(
        &mut *store,
        ListInRecord1Param {
            a: "list_in_record1",
        },
    )?;
    assert_eq!(
        exports.call_f_list_in_record2(&mut *store)?.a,
        "list_in_record2"
    );

    assert_eq!(
        exports
            .call_f_list_in_record3(
                &mut *store,
                ListInRecord3Param {
                    a: "list_in_record3 input"
                }
            )?
            .a,
        "list_in_record3 output"
    );

    assert_eq!(
        exports
            .call_f_list_in_record4(&mut *store, ListInAliasParam { a: "input4" })?
            .a,
        "result4"
    );

    exports.call_f_list_in_variant1(
        &mut *store,
        Some("foo"),
        Err("bar"),
        ListInVariant1V3Param::String("baz"),
    )?;
    assert_eq!(
        exports.call_f_list_in_variant2(&mut *store)?,
        Some("list_in_variant2".to_string())
    );
    assert_eq!(
        exports.call_f_list_in_variant3(&mut *store, Some("input3"))?,
        Some("output3".to_string())
    );

    assert!(exports.call_errno_result(&mut *store)?.is_err());
    MyErrno::A.to_string();
    format!("{:?}", MyErrno::A);
    fn assert_error<T: std::error::Error>() {}
    assert_error::<MyErrno>();

    let (a, b) = exports.call_list_typedefs(&mut *store, "typedef1", &["typedef2"])?;
    assert_eq!(a, b"typedef3");
    assert_eq!(b.len(), 1);
    assert_eq!(b[0], "typedef4");
    Ok(())
}
