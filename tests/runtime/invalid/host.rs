wit_bindgen_host_wasmtime_rust::generate!("../../tests/runtime/invalid/world.wit");

use anyhow::{Context, Result};
use imports::*;
use wasmtime::component::{Component, Linker};
use wasmtime::{Engine, Store};

#[derive(Default)]
pub struct MyImports;

impl Imports for MyImports {
    // The following types are truncated when out-of-bounds
    fn roundtrip_u8(&mut self, v: u8) -> Result<u8> {
        Ok(v)
    }
    fn roundtrip_s8(&mut self, v: i8) -> Result<i8> {
        Ok(v)
    }
    fn roundtrip_u16(&mut self, v: u16) -> Result<u16> {
        Ok(v)
    }
    fn roundtrip_s16(&mut self, v: i16) -> Result<i16> {
        Ok(v)
    }
    fn roundtrip_bool(&mut self, v: bool) -> Result<bool> {
        Ok(v)
    }

    // None of this should be reached and instead validation should prevent them
    // from being called
    fn roundtrip_char(&mut self, _: char) -> Result<char> {
        unreachable!()
    }
    fn roundtrip_enum(&mut self, _: imports::E) -> Result<imports::E> {
        unreachable!()
    }
    fn unaligned1(&mut self, _: Vec<u16>) -> Result<()> {
        unreachable!()
    }
    fn unaligned2(&mut self, _: Vec<u32>) -> Result<()> {
        unreachable!()
    }
    fn unaligned3(&mut self, _: Vec<u64>) -> Result<()> {
        unreachable!()
    }
    fn unaligned4(&mut self, _: Vec<imports::Flag32>) -> Result<()> {
        unreachable!()
    }
    fn unaligned5(&mut self, _: Vec<imports::Flag64>) -> Result<()> {
        unreachable!()
    }
    fn unaligned6(&mut self, _: Vec<imports::UnalignedRecord>) -> Result<()> {
        unreachable!()
    }
    fn unaligned7(&mut self, _: Vec<f32>) -> Result<()> {
        unreachable!()
    }
    fn unaligned8(&mut self, _: Vec<f64>) -> Result<()> {
        unreachable!()
    }
    fn unaligned9(&mut self, _: Vec<String>) -> Result<()> {
        unreachable!()
    }
    fn unaligned10(&mut self, _: Vec<Vec<u8>>) -> Result<()> {
        unreachable!()
    }
}

fn run(wasm: &str) -> Result<()> {
    let engine = Engine::new(&crate::default_config()?)?;
    let module = Component::from_file(&engine, wasm)?;

    let mut linker = Linker::new(&engine);
    imports::add_to_linker(&mut linker, |cx: &mut crate::Context<MyImports>| {
        &mut cx.imports
    })?;
    crate::testwasi::add_to_linker(&mut linker, |cx| &mut cx.testwasi)?;

    let mut store = Store::new(&engine, Default::default());

    let exports = Invalid::instantiate(&mut store, &module, &linker)?.0;
    exports.invalid_bool(&mut store)?;
    exports.invalid_u8(&mut store)?;
    exports.invalid_s8(&mut store)?;
    exports.invalid_u16(&mut store)?;
    exports.invalid_s16(&mut store)?;

    let mk = |store: &mut Store<_>| Invalid::instantiate(store, &module, &linker).map(|p| p.0);

    assert_err(
        mk(&mut store)?.invalid_char(&mut store),
        "converted integer out of range for `char`",
    )?;
    assert_err(
        mk(&mut store)?.invalid_enum(&mut store),
        "unexpected discriminant: ",
    )?;
    assert_err(mk(&mut store)?.unaligned1(&mut store), "not aligned")?;
    assert_err(mk(&mut store)?.unaligned2(&mut store), "not aligned")?;
    assert_err(mk(&mut store)?.unaligned3(&mut store), "not aligned")?;
    assert_err(mk(&mut store)?.unaligned4(&mut store), "not aligned")?;
    assert_err(mk(&mut store)?.unaligned5(&mut store), "not aligned")?;
    assert_err(mk(&mut store)?.unaligned6(&mut store), "not aligned")?;
    assert_err(mk(&mut store)?.unaligned7(&mut store), "not aligned")?;
    assert_err(mk(&mut store)?.unaligned8(&mut store), "not aligned")?;
    assert_err(mk(&mut store)?.unaligned9(&mut store), "not aligned")?;
    assert_err(mk(&mut store)?.unaligned10(&mut store), "not aligned")?;

    return Ok(());

    fn assert_err(result: Result<()>, err: &str) -> Result<()> {
        match result {
            Ok(()) => anyhow::bail!("export didn't trap"),
            Err(e) => {
                if format!("{e:?}").contains(err) {
                    Ok(())
                } else {
                    Err(e).with_context(|| format!("expected trap containing \"{}\"", err))
                }
            }
        }
    }
}
