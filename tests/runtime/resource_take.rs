use wasmtime::Store;

wasmtime::component::bindgen!(in "tests/runtime/resource_take");

use exports::test::resource_take::test::Test;

#[test]
fn run() -> anyhow::Result<()> {
    crate::run_test(
        "resource_take",
        |_| Ok(()),
        |store, component, linker| {
            let (u, e) = ResourceTake::instantiate(store, component, linker)?;
            Ok((u.interface0, e))
        },
        run_test,
    )
}

fn run_test(instance: Test, store: &mut Store<crate::Wasi<()>>) -> anyhow::Result<()> {
    instance.call_test(&mut *store)
}
