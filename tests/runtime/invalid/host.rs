wit_bindgen_host_wasmtime_rust::export!("../../tests/runtime/invalid/imports.wit");

use anyhow::Result;
use imports::*;
use wasmtime::Trap;

#[derive(Default)]
pub struct MyImports;

impl Imports for MyImports {
    type HostState = ();

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
    fn get_internal(&mut self, _: &()) -> u32 {
        unreachable!()
    }
}

wit_bindgen_host_wasmtime_rust::import!("../../tests/runtime/invalid/exports.wit");

fn run(wasm: &str) -> Result<()> {
    use exports::*;

    let (exports, mut store) = crate::instantiate(
        wasm,
        |linker| {
            imports::add_to_linker(
                linker,
                |cx: &mut crate::Context<(MyImports, imports::ImportsTables<MyImports>), _>| {
                    (&mut cx.imports.0, &mut cx.imports.1)
                },
            )
        },
        |store, module, linker| Exports::instantiate(store, module, linker, |cx| &mut cx.exports),
    )?;

    assert_err(
        exports.invalid_bool(&mut store),
        "invalid discriminant for `bool`",
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
    assert_err(exports.invalid_handle(&mut store), "invalid handle index")?;
    assert_err(
        exports.invalid_handle_close(&mut store),
        "invalid handle index",
    )?;
    return Ok(());

    fn assert_err(result: Result<(), Trap>, err: &str) -> Result<()> {
        match result {
            Ok(()) => anyhow::bail!("export didn't trap"),
            Err(e) if e.to_string().contains(err) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
