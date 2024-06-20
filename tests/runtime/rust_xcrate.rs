use anyhow::Result;
use wasmtime::component::Resource;
use wasmtime::Store;

wasmtime::component::bindgen!("c" in "tests/runtime/rust_xcrate");

use test::xcrate::a_imports::X as A_X;
use test::xcrate::b_imports::X as B_X;

#[derive(Default)]
pub struct MyImports;

impl test::xcrate::a_imports::Host for MyImports {
    fn a(&mut self) {}
}

impl test::xcrate::a_imports::HostX for MyImports {
    fn new(&mut self) -> Resource<A_X> {
        Resource::new_own(2)
    }

    fn foo(&mut self, _resource: Resource<A_X>) {}

    fn drop(&mut self, _resource: Resource<A_X>) -> Result<()> {
        Ok(())
    }
}

impl test::xcrate::b_imports::Host for MyImports {
    fn b(&mut self) {}
}

impl test::xcrate::b_imports::HostX for MyImports {
    fn new(&mut self) -> Resource<B_X> {
        Resource::new_own(2)
    }

    fn foo(&mut self, _resource: Resource<B_X>) {}

    fn drop(&mut self, _resource: Resource<B_X>) -> Result<()> {
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

    let x = exports.an_exported_interface().x();
    let resource = x.call_constructor(&mut *store)?;
    x.call_foo(&mut *store, resource.clone())?;
    resource.resource_drop(&mut *store)?;

    Ok(())
}
