use anyhow::Result;

wit_bindgen_host_wasmtime_rust::generate!({
    import: "../../tests/runtime/flavorful/imports.wit",
    default: "../../tests/runtime/flavorful/exports.wit",
    tracing: true,
    name: "exports",
});

#[derive(Default)]
pub struct MyImports;

impl imports::Imports for MyImports {
    fn f_list_in_record1(&mut self, ty: imports::ListInRecord1) {
        assert_eq!(ty.a, "list_in_record1");
    }

    fn f_list_in_record2(&mut self) -> imports::ListInRecord2 {
        imports::ListInRecord2 {
            a: "list_in_record2".to_string(),
        }
    }

    fn f_list_in_record3(&mut self, a: imports::ListInRecord3) -> imports::ListInRecord3 {
        assert_eq!(a.a, "list_in_record3 input");
        imports::ListInRecord3 {
            a: "list_in_record3 output".to_string(),
        }
    }

    fn f_list_in_record4(&mut self, a: imports::ListInAlias) -> imports::ListInAlias {
        assert_eq!(a.a, "input4");
        imports::ListInRecord4 {
            a: "result4".to_string(),
        }
    }

    fn f_list_in_variant1(
        &mut self,
        a: imports::ListInVariant1V1,
        b: imports::ListInVariant1V2,
        c: imports::ListInVariant1V3,
    ) {
        assert_eq!(a.unwrap(), "foo");
        assert_eq!(b.unwrap_err(), "bar");
        match c {
            imports::ListInVariant1V3::String(s) => assert_eq!(s, "baz"),
            imports::ListInVariant1V3::F32(_) => panic!(),
        }
    }

    fn f_list_in_variant2(&mut self) -> Option<String> {
        Some("list_in_variant2".to_string())
    }

    fn f_list_in_variant3(&mut self, a: imports::ListInVariant3) -> Option<String> {
        assert_eq!(a.unwrap(), "input3");
        Some("output3".to_string())
    }

    fn errno_result(&mut self) -> Result<(), imports::MyErrno> {
        imports::MyErrno::A.to_string();
        format!("{:?}", imports::MyErrno::A);
        fn assert_error<T: std::error::Error>() {}
        assert_error::<imports::MyErrno>();
        Err(imports::MyErrno::B)
    }

    fn list_typedefs(
        &mut self,
        a: imports::ListTypedef,
        b: imports::ListTypedef3,
    ) -> (imports::ListTypedef2, imports::ListTypedef3) {
        assert_eq!(a, "typedef1");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0], "typedef2");
        (b"typedef3".to_vec(), vec!["typedef4".to_string()])
    }

    fn list_of_variants(
        &mut self,
        bools: Vec<bool>,
        results: Vec<Result<(), ()>>,
        enums: Vec<imports::MyErrno>,
    ) -> (Vec<bool>, Vec<Result<(), ()>>, Vec<imports::MyErrno>) {
        assert_eq!(bools, [true, false]);
        assert_eq!(results, [Ok(()), Err(())]);
        assert_eq!(enums, [imports::MyErrno::Success, imports::MyErrno::A]);
        (
            vec![false, true],
            vec![Err(()), Ok(())],
            vec![imports::MyErrno::A, imports::MyErrno::B],
        )
    }
}

fn run(wasm: &str) -> Result<()> {
    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| imports::add_to_linker(linker, |cx| -> &mut MyImports { &mut cx.imports }),
        |store, module, linker| Exports::instantiate(store, module, linker),
    )?;

    exports.test_imports(&mut store)?;

    exports.f_list_in_record1(
        &mut store,
        ListInRecord1 {
            a: "list_in_record1",
        },
    )?;
    assert_eq!(exports.f_list_in_record2(&mut store)?.a, "list_in_record2");

    assert_eq!(
        exports
            .f_list_in_record3(
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
            .f_list_in_record4(&mut store, ListInAliasParam { a: "input4" })?
            .a,
        "result4"
    );

    exports.f_list_in_variant1(
        &mut store,
        Some("foo"),
        Err("bar"),
        ListInVariant1V3::String("baz"),
    )?;
    assert_eq!(
        exports.f_list_in_variant2(&mut store)?,
        Some("list_in_variant2".to_string())
    );
    assert_eq!(
        exports.f_list_in_variant3(&mut store, Some("input3"))?,
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
