use anyhow::Result;
use wasmtime::{Instance, Store};

witx_bindgen_wasmtime::export!("tests/wasm.witx");

use crate::Context;
use wasm::*;

pub(crate) use wasm::{Wasm, WasmData};

pub fn test(wasm: &Wasm<Context>, instance: Instance, store: &mut Store<Context>) -> Result<()> {
    let bytes = wasm.allocated_bytes(&mut *store)?;
    wasm.run_import_tests(&mut *store)?;
    scalars(wasm, store)?;
    records(wasm, store)?;
    variants(wasm, store)?;
    lists(wasm, store)?;
    flavorful(wasm, store)?;
    invalid(&instance, store)?;
    // buffers(wasm)?;
    handles(wasm, store)?;

    // Ensure that we properly called `free` everywhere in all the glue that we
    // needed to.
    assert_eq!(bytes, wasm.allocated_bytes(&mut *store)?);
    Ok(())
}

fn flavorful(wasm: &Wasm<Context>, store: &mut Store<Context>) -> Result<()> {
    Ok(())
}

fn invalid(i: &Instance, store: &mut Store<Context>) -> Result<()> {
    run_err(i, store, "invalid_bool", "invalid discriminant for `bool`")?;
    run_err(i, store, "invalid_u8", "out-of-bounds integer conversion")?;
    run_err(i, store, "invalid_s8", "out-of-bounds integer conversion")?;
    run_err(i, store, "invalid_u16", "out-of-bounds integer conversion")?;
    run_err(i, store, "invalid_s16", "out-of-bounds integer conversion")?;
    run_err(i, store, "invalid_char", "char value out of valid range")?;
    run_err(i, store, "invalid_e1", "invalid discriminant for `E1`")?;
    run_err(i, store, "invalid_handle", "invalid handle index")?;
    run_err(i, store, "invalid_handle_close", "invalid handle index")?;
    return Ok(());

    fn run_err(i: &Instance, store: &mut Store<Context>, name: &str, err: &str) -> Result<()> {
        match run(i, store, name) {
            Ok(()) => anyhow::bail!("export `{}` didn't trap", name),
            Err(e) if e.to_string().contains(err) => Ok(()),
            Err(e) => Err(e),
        }
    }

    fn run(i: &Instance, store: &mut Store<Context>, name: &str) -> Result<()> {
        let run = i.get_typed_func::<(), (), _>(&mut *store, name)?;
        run.call(store, ())?;
        Ok(())
    }
}
