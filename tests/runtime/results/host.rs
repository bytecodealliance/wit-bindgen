use wit_bindgen_host_wasmtime_rust::Result as HostResult;
wit_bindgen_host_wasmtime_rust::generate!("../../tests/runtime/results/world.wit");

#[derive(Default)]
pub struct MyImports {}

#[derive(Debug)]
struct MyTrap;
impl std::fmt::Display for MyTrap {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "my very own trap")
    }
}
impl std::error::Error for MyTrap {}

impl imports::Imports for MyImports {
    // The interface error type is a String, which is a primitive, therefore
    // we need to use an outer anyhow::Result for trapping, and an inner
    // Result<f32, String> to represent the interface result.
    fn string_error(&mut self, a: f32) -> anyhow::Result<Result<f32, String>> {
        if a == 0.0 {
            Ok(Err("zero".to_owned()))
        } else {
            Ok(Ok(a))
        }
    }

    // The interface error type is defined (as an enum), therefore
    // wit-bindgen-host-wasmtime-rust will impl all the traits to make it a
    // std::error::Error, as well as an `impl From<E> for
    // wit_bindgen_host_wasmtime_rust::Error<E>`. This means we can use `?` to
    // covert a Result<_, E> into a HostResult<_, E>.
    //
    // We expect a lot of wit interfaces to look like this one.
    fn enum_error(&mut self, a: f64) -> HostResult<f64, imports::E> {
        if a == 0.0 {
            Err(imports::E::A)?
        } else {
            Ok(a)
        }
    }

    // Same ideas as enum_error, but the interface error is defined as a
    // record.
    //
    // Shows how you can trap in a HostResult func with an ordinary anyhow::Error.
    fn record_error(&mut self, a: f64) -> HostResult<f64, imports::E2> {
        if a == 0.0 {
            Err(imports::E2 {
                line: 420,
                column: 0,
            })?
        } else if a == 1.0 {
            Err(anyhow::Error::msg("a somewhat ergonomic trap"))?
        } else {
            Ok(a)
        }
    }

    // Same ideas as enum_error, but the interface error is defined as a
    // variant.
    //
    // Shows how you can trap in a HostResult func with anything that impls
    // std::error::Error
    fn variant_error(&mut self, a: f64) -> HostResult<f64, imports::E3> {
        if a == 0.0 {
            Err(imports::E3::E2(imports::E2 {
                line: 420,
                column: 0,
            }))?
        } else if a == 1.0 {
            Err(imports::E3::E1(imports::E::B))?
        } else if a == 2.0 {
            Err(wit_bindgen_host_wasmtime_rust::Error::trap(MyTrap))?
        } else {
            Ok(a)
        }
    }

    // Finally, another case where we can't impl Error on the error type,
    // so we need a nested result.
    //
    // In this function body we show how the outer result does indeed trap
    // execution.
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
            |store, module, linker| Results::instantiate(store, module, linker),
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
    let e = exports.record_error(&mut store, 1.0);
    assert!(e.is_err());
    assert!(format!("{:?}", e.err().unwrap()).contains("a somewhat ergonomic trap"));

    let (exports, mut store) = create()?;
    assert!(exports.record_error(&mut store, 2.0)?.is_ok());

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
    assert!(format!("{:?}", e.err().unwrap()).contains("my very own trap"));

    let (exports, mut store) = create()?;
    assert_eq!(exports.empty_error(&mut store, 0)?, Err(()));

    let e = exports.empty_error(&mut store, 1);
    assert!(e.is_err());
    assert!(format!("{:?}", e.err().unwrap()).contains("outer result trap"));

    let (exports, mut store) = create()?;
    assert_eq!(exports.empty_error(&mut store, 2)?, Ok(2));

    Ok(())
}
