wit_bindgen_wasmer::export!("./tests/runtime/invalid/imports.wit");

use anyhow::Result;
use imports::*;
use wasmer::{RuntimeError, WasmerEnv};

#[derive(WasmerEnv, Clone)]
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

wit_bindgen_wasmer::import!("./tests/runtime/invalid/exports.wit");

fn run(wasm: &str) -> Result<()> {
    use exports::*;

    let exports = crate::instantiate(
        wasm,
        |store, import_object| imports::add_to_imports(store, import_object, MyImports),
        |store, module, import_object| exports::Exports::instantiate(store, module, import_object),
    )?;

    assert_err(exports.invalid_bool(), "invalid discriminant for `bool`")?;
    assert_err(exports.invalid_u8(), "out-of-bounds integer conversion")?;
    assert_err(exports.invalid_s8(), "out-of-bounds integer conversion")?;
    assert_err(exports.invalid_u16(), "out-of-bounds integer conversion")?;
    assert_err(exports.invalid_s16(), "out-of-bounds integer conversion")?;
    assert_err(exports.invalid_char(), "char value out of valid range")?;
    assert_err(exports.invalid_enum(), "invalid discriminant for `E`")?;
    assert_err(exports.invalid_handle(), "invalid handle index")?;
    assert_err(exports.invalid_handle_close(), "invalid handle index")?;
    return Ok(());

    fn assert_err(result: Result<(), RuntimeError>, err: &str) -> Result<()> {
        match result {
            Ok(()) => anyhow::bail!("export didn't trap"),
            Err(e) if e.to_string().contains(err) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
