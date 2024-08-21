use wasmtime::Store;

wasmtime::component::bindgen!(in "tests/runtime/resource_into_inner");

use exports::test::resource_into_inner::test::Guest;

#[test]
fn run() -> anyhow::Result<()> {
    crate::run_test(
        "resource_into_inner",
        |_| Ok(()),
        |store, component, linker| {
            Ok(ResourceIntoInner::instantiate(store, component, linker)?.interface0)
        },
        run_test,
    )
}

fn run_test(instance: Guest, store: &mut Store<crate::Wasi<()>>) -> anyhow::Result<()> {
    instance.call_test(&mut *store)
}
