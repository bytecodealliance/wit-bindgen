use anyhow::Result;
use wasmtime::Store;

wasmtime::component::bindgen!(in "tests/runtime/ownership");

#[derive(Default)]
pub struct MyImports {
    called_foo: bool,
    called_bar: bool,
    called_baz: bool,
}

impl lists::Host for MyImports {
    fn foo(&mut self, list: Vec<Vec<String>>) -> Result<Vec<Vec<String>>> {
        self.called_foo = true;
        Ok(list)
    }
}

impl thing_in::Host for MyImports {
    fn bar(&mut self, _value: thing_in::Thing) -> Result<()> {
        self.called_bar = true;
        Ok(())
    }
}

impl thing_in_and_out::Host for MyImports {
    fn baz(&mut self, value: thing_in_and_out::Thing) -> Result<thing_in_and_out::Thing> {
        self.called_baz = true;
        Ok(value)
    }
}

#[test]
fn run() -> Result<()> {
    for name in ["owning", "borrowing", "borrowing-duplicate-if-necessary"] {
        crate::run_test_from_dir(
            "ownership",
            name,
            |linker| Ownership::add_to_linker(linker, |x| &mut x.0),
            |store, component, linker| Ownership::instantiate(store, component, linker),
            run_test,
        )?;
    }

    Ok(())
}

fn run_test(exports: Ownership, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    exports.call_foo(&mut *store)?;

    assert!(store.data().0.called_foo);
    assert!(store.data().0.called_bar);
    assert!(store.data().0.called_baz);

    Ok(())
}
