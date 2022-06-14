use anyhow::Result;

wit_bindgen_wasmtime::export!("../../tests/runtime/flavorful/imports.wit");

use imports::*;

#[derive(Default)]
pub struct MyImports;

impl Imports for MyImports {
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
        a: ListInVariant1V1<'_>,
        b: ListInVariant1V2<'_>,
        c: ListInVariant1V3<'_>,
    ) {
        assert_eq!(a.unwrap(), "foo");
        assert_eq!(b.unwrap_err(), "bar");
        match c {
            ListInVariant1V3::String(s) => assert_eq!(s, "baz"),
            ListInVariant1V3::F32(_) => panic!(),
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

wit_bindgen_wasmtime::import!("../../tests/runtime/flavorful/exports.wit");

fn run(wasm: &str) -> Result<()> {
    use exports::*;

    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| imports::add_to_linker(linker, |cx| -> &mut MyImports { &mut cx.imports }),
        |store, module, linker| Exports::instantiate(store, module, linker, |cx| &mut cx.exports),
    )?;

    exports.test_imports(&mut store)?;

    exports.list_in_record1(
        &mut store,
        ListInRecord1 {
            a: "list_in_record1",
        },
    )?;
    assert_eq!(exports.list_in_record2(&mut store)?.a, "list_in_record2");

    assert_eq!(
        exports
            .list_in_record3(
                &mut store,
                ListInRecord3Param {
                    a: "list_in_record3 input"
                }
            )?
            .a,
        "list_in_record3 output"
    );

    assert_eq!(
        exports
            .list_in_record4(&mut store, ListInAliasParam { a: "input4" })?
            .a,
        "result4"
    );

    exports.list_in_variant1(
        &mut store,
        Some("foo"),
        Err("bar"),
        ListInVariant1V3::String("baz"),
    )?;
    assert_eq!(
        exports.list_in_variant2(&mut store)?,
        Some("list_in_variant2".to_string())
    );
    assert_eq!(
        exports.list_in_variant3(&mut store, Some("input3"))?,
        Some("output3".to_string())
    );

    assert!(exports.errno_result(&mut store)?.is_err());
    MyErrno::A.to_string();
    format!("{:?}", MyErrno::A);
    fn assert_error<T: std::error::Error>() {}
    assert_error::<MyErrno>();

    let (a, b) = exports.list_typedefs(&mut store, "typedef1", &["typedef2"])?;
    assert_eq!(a, b"typedef3");
    assert_eq!(b.len(), 1);
    assert_eq!(b[0], "typedef4");
    Ok(())
}
