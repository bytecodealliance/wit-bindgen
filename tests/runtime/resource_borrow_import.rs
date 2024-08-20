use std::collections::HashMap;
use wasmtime::{component::Resource, Store};

wasmtime::component::bindgen!(in "tests/runtime/resource_borrow_import");

use test::resource_borrow_import::test::{Host, HostThing, Thing};

#[derive(Default)]

pub struct MyHostThing {
    map_l: HashMap<u32, u32>,
    next_id: u32,
}

impl HostThing for MyHostThing {
    fn new(&mut self, v: u32) -> wasmtime::component::Resource<Thing> {
        let id = self.next_id;
        self.next_id += 1;
        self.map_l.insert(id, v + 2);
        Resource::new_own(id)
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Thing>) -> wasmtime::Result<()> {
        let id = rep.rep();
        self.map_l.remove(&id);
        Ok(())
    }
}

impl Host for MyHostThing {
    fn foo(&mut self, v: wasmtime::component::Resource<Thing>) -> u32 {
        let id = v.rep();
        self.map_l[&id] + 3
    }
}

#[test]
fn run() -> anyhow::Result<()> {
    crate::run_test(
        "resource_borrow_import",
        |linker| ResourceBorrowImport::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| ResourceBorrowImport::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(
    instance: ResourceBorrowImport,
    store: &mut Store<crate::Wasi<MyHostThing>>,
) -> anyhow::Result<()> {
    let res = instance.call_test(&mut *store, 42)?;
    assert_eq!(res, 42 + 1 + 2 + 3 + 4);

    Ok(())
}
