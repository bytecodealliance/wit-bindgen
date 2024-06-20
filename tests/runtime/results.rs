use anyhow::Result;
use wasmtime::Store;

wasmtime::component::bindgen!({
    path: "tests/runtime/results",
});

use test::results::test as imports;

#[derive(Default)]
pub struct MyImports;

impl test::results::test::Host for MyImports {
    fn string_error(&mut self, a: f32) -> Result<f32, String> {
        if a == 0.0 {
            Err("zero".to_owned())
        } else {
            Ok(a)
        }
    }

    fn enum_error(&mut self, a: f32) -> Result<f32, imports::E> {
        if a == 0.0 {
            Err(imports::E::A)
        } else {
            Ok(a)
        }
    }

    fn record_error(&mut self, a: f32) -> Result<f32, imports::E2> {
        if a == 0.0 {
            Err(imports::E2 {
                line: 420,
                column: 0,
            })
        } else if a == 1.0 {
            Err(imports::E2 {
                line: 77,
                column: 2,
            })
        } else {
            Ok(a)
        }
    }

    fn variant_error(&mut self, a: f32) -> Result<f32, imports::E3> {
        if a == 0.0 {
            Err(imports::E3::E2(imports::E2 {
                line: 420,
                column: 0,
            }))
        } else if a == 1.0 {
            Err(imports::E3::E1(imports::E::B))
        } else if a == 2.0 {
            Err(imports::E3::E1(imports::E::C))
        } else {
            Ok(a)
        }
    }

    fn empty_error(&mut self, a: u32) -> Result<u32, ()> {
        if a == 0 {
            Err(())
        } else if a == 1 {
            Ok(42)
        } else {
            Ok(a)
        }
    }

    fn double_error(&mut self, a: u32) -> Result<Result<(), String>, String> {
        if a == 0 {
            Ok(Ok(()))
        } else if a == 1 {
            Ok(Err("one".into()))
        } else {
            Err("two".into())
        }
    }
}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "results",
        |linker| Results::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| Results::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(results: Results, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    use exports::test::results::test::{E, E2, E3};

    assert_eq!(
        results.interface0.call_string_error(&mut *store, 0.0)?,
        Err("zero".to_owned())
    );
    assert_eq!(
        results.interface0.call_string_error(&mut *store, 1.0)?,
        Ok(1.0)
    );

    assert_eq!(
        results.interface0.call_enum_error(&mut *store, 0.0)?,
        Err(E::A)
    );
    assert_eq!(
        results.interface0.call_enum_error(&mut *store, 0.0)?,
        Err(E::A)
    );

    assert!(matches!(
        results.interface0.call_record_error(&mut *store, 0.0)?,
        Err(E2 {
            line: 420,
            column: 0
        })
    ));
    assert!(matches!(
        results.interface0.call_record_error(&mut *store, 1.0)?,
        Err(E2 {
            line: 77,
            column: 2
        })
    ));

    assert!(results
        .interface0
        .call_record_error(&mut *store, 2.0)?
        .is_ok());

    assert!(matches!(
        results.interface0.call_variant_error(&mut *store, 0.0)?,
        Err(E3::E2(E2 {
            line: 420,
            column: 0
        }))
    ));
    assert!(matches!(
        results.interface0.call_variant_error(&mut *store, 1.0)?,
        Err(E3::E1(E::B))
    ));
    assert!(matches!(
        results.interface0.call_variant_error(&mut *store, 2.0)?,
        Err(E3::E1(E::C))
    ));

    assert_eq!(
        results.interface0.call_empty_error(&mut *store, 0)?,
        Err(())
    );
    assert_eq!(results.interface0.call_empty_error(&mut *store, 1)?, Ok(42));
    assert_eq!(results.interface0.call_empty_error(&mut *store, 2)?, Ok(2));

    assert_eq!(
        results.interface0.call_double_error(&mut *store, 0)?,
        Ok(Ok(()))
    );
    assert_eq!(
        results.interface0.call_double_error(&mut *store, 1)?,
        Ok(Err("one".into()))
    );
    assert_eq!(
        results.interface0.call_double_error(&mut *store, 2)?,
        Err("two".into())
    );

    Ok(())
}
