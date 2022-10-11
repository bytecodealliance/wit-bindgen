wit_bindgen_host_wasmtime_rust::export!("../../tests/runtime/invalid/imports.wit");

use anyhow::{Context, Result};
use imports::*;
use wasmtime::Trap;

#[derive(Default)]
pub struct MyImports;

impl Imports for MyImports {
    // The following types are truncated when out-of-bounds
    fn roundtrip_u8(&mut self, v: u8) -> u8 {
        v
    }
    fn roundtrip_s8(&mut self, v: i8) -> i8 {
        v
    }
    fn roundtrip_u16(&mut self, v: u16) -> u16 {
        v
    }
    fn roundtrip_s16(&mut self, v: i16) -> i16 {
        v
    }
    fn roundtrip_bool(&mut self, v: bool) -> bool {
        v
    }

    // These values trap when out-of-bounds
    fn roundtrip_char(&mut self, _: char) -> char {
        unreachable!()
    }
    fn roundtrip_enum(&mut self, _: imports::E) -> imports::E {
        unreachable!()
    }

    // These values trap when unaligned
    fn unaligned_roundtrip1(
        &mut self,
        _u16s: Vec<u16>,
        _u32s: Vec<u32>,
        _u64s: Vec<u64>,
        _flag32s: Vec<Flag32>,
        _flag64s: Vec<Flag64>,
    ) {
        unreachable!()
    }

    fn unaligned_roundtrip2(
        &mut self,
        _records: Vec<UnalignedRecord>,
        _f32s: Vec<f32>,
        _f64s: Vec<f64>,
        _strings: Vec<String>,
        _lists: Vec<Vec<u8>>,
    ) {
        unreachable!()
    }
}

wit_bindgen_host_wasmtime_rust::import!("../../tests/runtime/invalid/exports.wit");

fn run(wasm: &str) -> Result<()> {
    use exports::*;

    let mkstore = || {
        crate::instantiate(
            wasm,
            |linker| {
                imports::add_to_linker(linker, |cx: &mut crate::Context<MyImports>| &mut cx.imports)
            },
            |store, module, linker| Exports::instantiate(store, module, linker),
        )
    };

    let (exports, mut store) = mkstore()?;

    exports.invalid_bool(&mut store)?;
    exports.invalid_u8(&mut store)?;
    exports.invalid_s8(&mut store)?;
    exports.invalid_u16(&mut store)?;
    exports.invalid_s16(&mut store)?;

    assert_err(
        exports.invalid_char(&mut store),
        "converted integer out of range for `char`",
    )?;

    // After a trap, can't re-enter instance, so create a new one:
    let (exports, mut store) = mkstore()?;
    assert_err(
        exports.invalid_enum(&mut store),
        "unexpected discriminant: ",
    )?;

    let (exports, mut store) = mkstore()?;
    assert_err(exports.test_unaligned(&mut store), "is not aligned")?;

    return Ok(());

    fn assert_err(result: Result<(), anyhow::Error>, err: &str) -> Result<()> {
        match result {
            Ok(()) => anyhow::bail!("export didn't trap"),
            Err(e) => match e.downcast_ref::<Trap>() {
                Some(e) if e.to_string().contains(err) => Ok(()),
                Some(_) | None => {
                    Err(e).with_context(|| format!("expected trap containing \"{}\"", err))
                }
            },
        }
    }
}
