use wit_bindgen_host_wasmtime_rust::Result as HostResult;
wit_bindgen_host_wasmtime_rust::generate!({
    import: "../../tests/runtime/results/imports.wit",
    default: "../../tests/runtime/results/exports.wit",
    name: "exports",
});

#[derive(Default)]
pub struct MyImports {}

impl imports::Imports for MyImports {
    fn string_error(&mut self, a: f32) -> anyhow::Result<Result<f32, String>> {
        if a == 0.0 {
            Ok(Err("zero".to_owned()))
        } else {
            Ok(Ok(a))
        }
    }

    fn enum_error(&mut self, a: f64) -> HostResult<f64, imports::E> {
        if a == 0.0 {
            Err(imports::E::A)?
        } else {
            Ok(a)
        }
    }

    fn record_error(&mut self, a: f64) -> HostResult<f64, imports::E2> {
        if a == 0.0 {
            Err(imports::E2 {
                line: 420,
                column: 0,
            })?
        } else {
            Ok(a)
        }
    }

    fn variant_error(&mut self, a: f64) -> HostResult<f64, imports::E3> {
        if a == 0.0 {
            Err(imports::E3::E2(imports::E2 {
                line: 420,
                column: 0,
            }))?
        } else if a == 1.0 {
            Err(imports::E3::E1(imports::E::B))?
        } else if a == 2.0 {
            Err(anyhow::Error::msg("a somewhat ergonomic trap"))?
        } else {
            Ok(a)
        }
    }

    fn empty_error(&mut self, a: u32) -> anyhow::Result<Result<u32, ()>> {
        if a == 0 {
            Ok(Err(()))
        } else if a == 1 {
            Err(anyhow::Error::msg("outer result trap"))
        } else {
            Ok(Ok(a))
        }
    }
}

fn run(wasm: &str) -> anyhow::Result<()> {
    let create = || {
        crate::instantiate(
            wasm,
            |linker| {
                imports::add_to_linker(
                    linker,
                    |cx: &mut crate::Context<MyImports>| -> &mut MyImports { &mut cx.imports },
                )
            },
            |store, module, linker| Exports::instantiate(store, module, linker),
        )
    };

    let (exports, mut store) = create()?;

    assert_eq!(
        exports.string_error(&mut store, 0.0)?,
        Err("zero".to_owned())
    );
    assert_eq!(exports.string_error(&mut store, 1.0)?, Ok(1.0));

    assert_eq!(exports.enum_error(&mut store, 0.0)?, Err(E::A));
    assert_eq!(exports.enum_error(&mut store, 0.0)?, Err(E::A));

    assert!(matches!(
        exports.record_error(&mut store, 0.0)?,
        Err(E2 {
            line: 420,
            column: 0
        })
    ));
    assert!(exports.record_error(&mut store, 1.0)?.is_ok());

    assert!(matches!(
        exports.variant_error(&mut store, 0.0)?,
        Err(E3::E2(E2 {
            line: 420,
            column: 0
        }))
    ));
    assert!(matches!(
        exports.variant_error(&mut store, 1.0)?,
        Err(E3::E1(E::B))
    ));
    let e = exports.variant_error(&mut store, 2.0);
    assert!(e.is_err());
    assert!(e
        .err()
        .unwrap()
        .to_string()
        .starts_with("a somewhat ergonomic trap"));

    let (exports, mut store) = create()?;
    assert_eq!(exports.empty_error(&mut store, 0)?, Err(()));

    let e = exports.empty_error(&mut store, 1);
    assert!(e.is_err());
    assert!(e
        .err()
        .unwrap()
        .to_string()
        .starts_with("outer result trap"));

    let (exports, mut store) = create()?;
    assert_eq!(exports.empty_error(&mut store, 2)?, Ok(2));

    Ok(())
}
