wit_bindgen_host_wasmtime_rust::export!("../../tests/runtime/invalid/imports.wit");

use anyhow::{Context, Result};
use imports::*;
use wasmtime::Trap;

#[derive(Default)]
pub struct MyImports;

impl Imports for MyImports {
    fn roundtrip_u8(&mut self, _: u8) -> u8 {
        unreachable!()
    }
    fn roundtrip_s8(&mut self, _: i8) -> i8 {
        unreachable!()
    }
    fn roundtrip_u16(&mut self, _: u16) -> u16 {
        unreachable!()
    }
    fn roundtrip_s16(&mut self, _: i16) -> i16 {
        unreachable!()
    }
    fn roundtrip_char(&mut self, _: char) -> char {
        unreachable!()
    }
    fn roundtrip_bool(&mut self, _: bool) -> bool {
        unreachable!()
    }
    fn roundtrip_enum(&mut self, _: imports::E) -> imports::E {
        unreachable!()
    }

    fn unaligned_roundtrip1(
        &mut self,
        u16s: Vec<u16>,
        u32s: Vec<u32>,
        u64s: Vec<u64>,
        flag32s: Vec<Flag32>,
        flag64s: Vec<Flag64>,
    ) {
        assert_eq!(u16s, [1]);
        assert_eq!(u32s, [2]);
        assert_eq!(u64s, [3]);
        assert_eq!(flag32s, [Flag32::B8]);
        assert_eq!(flag64s, [Flag64::B9]);
    }

    fn unaligned_roundtrip2(
        &mut self,
        records: Vec<UnalignedRecord>,
        f32s: Vec<f32>,
        f64s: Vec<f64>,
        strings: Vec<String>,
        lists: Vec<Vec<u8>>,
    ) {
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].a, 10);
        assert_eq!(records[0].b, 11);
        assert_eq!(f32s, [100.0]);
        assert_eq!(f64s, [101.0]);
        assert_eq!(strings, ["foo"]);
        assert_eq!(lists, [&[102][..]]);
    }
}

wit_bindgen_host_wasmtime_rust::import!("../../tests/runtime/invalid/exports.wit");

fn run(wasm: &str) -> Result<()> {
    use exports::*;

    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| {
            imports::add_to_linker(linker, |cx: &mut crate::Context<MyImports>| &mut cx.imports)
        },
        |store, module, linker| Exports::instantiate(store, module, linker),
    )?;

    assert_err(
        exports.invalid_bool(&mut store),
        "out-of-bounds value for bool",
    )?;
    assert_err(
        exports.invalid_u8(&mut store),
        "out-of-bounds integer conversion",
    )?;
    assert_err(
        exports.invalid_s8(&mut store),
        "out-of-bounds integer conversion",
    )?;
    assert_err(
        exports.invalid_u16(&mut store),
        "out-of-bounds integer conversion",
    )?;
    assert_err(
        exports.invalid_s16(&mut store),
        "out-of-bounds integer conversion",
    )?;
    assert_err(
        exports.invalid_char(&mut store),
        "char value out of valid range",
    )?;
    assert_err(
        exports.invalid_enum(&mut store),
        "invalid discriminant for `E`",
    )?;

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
