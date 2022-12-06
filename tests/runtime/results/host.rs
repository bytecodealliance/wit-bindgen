wit_bindgen_host_wasmtime_rust::generate!({
    path: "../../tests/runtime/results/world.wit",
    trappable_error_type: {
        e => TrappableE,
        e2 => TrappableE2,
        e3 => TrappableE3,
    }
});

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

// It is possible to write these (very basic) From impls to get nice error handling
// from your locally defined traps.
impl From<MyTrap> for imports::TrappableE {
    fn from(t: MyTrap) -> imports::TrappableE {
        imports::TrappableE::trap(anyhow::Error::new(t))
    }
}

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

    // The interface error type E is defined (as an enum), and it will convert
    // to a TrappableE using the generated From impl.
    //
    // We expect a lot of wit interfaces to look like this one.
    fn enum_error(&mut self, a: f64) -> Result<f64, imports::TrappableE> {
        if a == 0.0 {
            Err(imports::E::A)?
        } else if a == 1.0 {
            // There is a hand-written From<MyTrap> for TrappableE, so we can trap with this
            // convenient shorthand:
            Err(MyTrap)?
        } else {
            Ok(a)
        }
    }

    // Same ideas as enum_error, but the interface error is defined as a
    // record.
    //
    fn record_error(&mut self, a: f64) -> Result<f64, imports::TrappableE2> {
        if a == 0.0 {
            Err(imports::E2 {
                line: 420,
                column: 0,
            })?
        } else if a == 1.0 {
            Err(imports::TrappableE2::trap(anyhow::anyhow!(
                "a somewhat ergonomic trap"
            )))?
        } else {
            Ok(a)
        }
    }

    // Same ideas as enum_error, but the interface error is defined as a
    // variant.
    //
    // Shows how you can trap in a HostResult func with anything that impls
    // std::error::Error
    fn variant_error(&mut self, a: f64) -> Result<f64, imports::TrappableE3> {
        if a == 0.0 {
            Err(imports::E3::E2(imports::E2 {
                line: 420,
                column: 0,
            }))?
        } else if a == 1.0 {
            Err(imports::E3::E1(imports::E::B))?
        } else if a == 2.0 {
            // If you don't write a From impl, you can still do this inline:
            Err(imports::TrappableE3::trap(anyhow::Error::new(MyTrap)))?
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
    exports
        .enum_error(&mut store, 1.0)
        .err()
        .expect("execution traps")
        .downcast::<MyTrap>()
        .expect("trap is a MyTrap");
    let (exports, mut store) = create()?;

    assert_eq!(exports.enum_error(&mut store, 2.0)?, Ok(2.0));

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
    let e = e.err().unwrap();
    assert!(e.downcast_ref::<MyTrap>().is_some());
    assert!(format!("{:?}", e).contains("my very own trap"));

    let (exports, mut store) = create()?;
    assert_eq!(exports.empty_error(&mut store, 0)?, Err(()));

    let e = exports.empty_error(&mut store, 1);
    assert!(e.is_err());
    assert!(format!("{:?}", e.err().unwrap()).contains("outer result trap"));

    let (exports, mut store) = create()?;
    assert_eq!(exports.empty_error(&mut store, 2)?, Ok(2));

    Ok(())
}
