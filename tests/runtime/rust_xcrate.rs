use anyhow::Result;
use wasmtime::Store;

wasmtime::component::bindgen!("c" in "tests/runtime/rust_xcrate");

#[derive(Default)]
pub struct MyImports;

impl test::xcrate::a_imports::Host for MyImports {
    fn a(&mut self) -> Result<()> {
        Ok(())
    }
}

impl test::xcrate::b_imports::Host for MyImports {
    fn b(&mut self) -> Result<()> {
        Ok(())
    }
}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "rust_xcrate",
        |linker| C::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| C::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(exports: C, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    exports.call_b(&mut *store)?;

    Ok(())
}
