use anyhow::{Ok, Result};
use wasmtime::Store;

wasmtime::component::bindgen!("required-exports" in "tests/runtime/type_section_suffix");
use self::test::suffix::imports::Host;

#[derive(Default)]
pub struct MyFoo;

impl Host for MyFoo {
    fn foo(&mut self) -> wasmtime::Result<()> {
        Ok(())
    }
}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "type_section_suffix",
        |linker| RequiredExports::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| RequiredExports::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(exports: RequiredExports, store: &mut Store<crate::Wasi<MyFoo>>) -> Result<()> {
    exports.call_run(&mut *store)?;
    Ok(())
}
