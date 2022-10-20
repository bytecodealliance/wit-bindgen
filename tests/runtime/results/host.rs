use wit_bindgen_host_wasmtime_rust::Result as HostResult;
wit_bindgen_host_wasmtime_rust::generate!({
    import: "../../tests/runtime/results/imports.wit",
    default: "../../tests/runtime/results/exports.wit",
    name: "exports",
});

#[derive(Default)]
pub struct MyImports {}

impl imports::Imports for MyImports {
    fn string_error(&mut self, a: f32) -> Result<f32, String> {
        if a == 0.0 {
            Err("zero".to_owned())
        } else {
            Ok(a)
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
        } else {
            Ok(a)
        }
    }

    fn empty_error(&mut self, a: u32) -> Result<u32, ()> {
        if a == 0 {
            Err(())?
        } else {
            Ok(a)
        }
    }
}

fn run(wasm: &str) -> anyhow::Result<()> {
    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| {
            imports::add_to_linker(
                linker,
                |cx: &mut crate::Context<MyImports>| -> &mut MyImports { &mut cx.imports },
            )
        },
        |store, module, linker| Exports::instantiate(store, module, linker),
    )?;

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

    assert_eq!(exports.empty_error(&mut store, 0)?, Err(()));
    assert_eq!(exports.empty_error(&mut store, 1)?, Ok(1));

    Ok(())
}
