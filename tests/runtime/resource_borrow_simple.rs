use wasmtime::{component::Resource, Store};

wasmtime::component::bindgen!(in "tests/runtime/resource_borrow_simple");

#[derive(Default)]

pub struct MyHostRImpl {}

impl HostR for MyHostRImpl {
    fn new(&mut self) -> wasmtime::component::Resource<R> {
        Resource::new_own(0)
    }

    fn drop(
        &mut self,
        _: wasmtime::component::Resource<R>,
    ) -> std::result::Result<(), anyhow::Error> {
        Ok(())
    }
}

impl ResourceBorrowSimpleImports for MyHostRImpl {
    fn test(&mut self, _: wasmtime::component::Resource<R>) {}
}

#[test]
fn run() -> anyhow::Result<()> {
    crate::run_test(
        "resource_borrow_simple",
        |linker| ResourceBorrowSimple::add_to_linker(linker, |x| &mut x.0),
        |store, component: &wasmtime::component::Component, linker| {
            ResourceBorrowSimple::instantiate(store, component, linker)
        },
        run_test,
    )
}

fn run_test(
    instance: ResourceBorrowSimple,
    store: &mut Store<crate::Wasi<MyHostRImpl>>,
) -> anyhow::Result<()> {
    instance.call_test_imports(&mut *store)?;
    Ok(())
}
