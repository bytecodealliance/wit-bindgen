use wasmtime::Store;

wasmtime::component::bindgen!(in "tests/runtime/resource_borrow_export");

use exports::test::resource_borrow_export::test::Guest;

#[test]
fn run() -> anyhow::Result<()> {
    crate::run_test(
        "resource_borrow_export",
        |_| Ok(()),
        |store, component, linker| {
            Ok(ResourceBorrowExport::instantiate(store, component, linker)?.interface0)
        },
        run_test,
    )
}

fn run_test(instance: Guest, store: &mut Store<crate::Wasi<()>>) -> anyhow::Result<()> {
    let thing = instance.thing().call_constructor(&mut *store, 42)?;
    let res = instance.call_foo(&mut *store, thing)?;
    assert_eq!(res, 42 + 1 + 2);
    Ok(())
}
