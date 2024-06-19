use std::collections::HashMap;
use wasmtime::{component::Resource, Store};

use self::{
    exports::test::resource_borrow_in_record::test::Foo as ImportFoo,
    test::resource_borrow_in_record::test::{Foo as HostFoo, Host as TestHost, HostThing, Thing},
};
use crate::resource_borrow_in_record::exports::test::resource_borrow_in_record::test::Guest;

wasmtime::component::bindgen!(in "tests/runtime/resource_borrow_in_record");

#[derive(Default)]
pub struct MyHostThing {
    map_l: HashMap<u32, String>,
    next_id: u32,
}

impl MyHostThing {
    pub fn get_value(&self, id: u32) -> &str {
        &self.map_l[&id]
    }
}

impl TestHost for MyHostThing {
    fn test(&mut self, a: Vec<HostFoo>) -> Vec<wasmtime::component::Resource<Thing>> {
        a.into_iter()
            .map(|a| {
                let val = self.get_value(a.thing.rep());
                // val + " test"
                HostThing::new(self, val.to_string() + " test")
            })
            .collect()
    }
}

impl HostThing for MyHostThing {
    fn new(&mut self, s: String) -> wasmtime::component::Resource<Thing> {
        let id = self.next_id;
        self.next_id += 1;
        self.map_l.insert(id, s + " HostThing");
        Resource::new_own(id)
    }

    fn get(&mut self, self_: wasmtime::component::Resource<Thing>) -> String {
        let id = self_.rep();
        self.map_l[&id].clone() + " HostThing.get"
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Thing>) -> wasmtime::Result<()> {
        let id = rep.rep();
        self.map_l.remove(&id);
        Ok(())
    }
}

#[test]
fn run() -> anyhow::Result<()> {
    crate::run_test(
        "resource_borrow_in_record",
        |linker| ResourceBorrowInRecord::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| {
            let (u, e) = ResourceBorrowInRecord::instantiate(store, component, linker)?;
            Ok((u.interface0, e))
        },
        run_test,
    )
}

fn run_test(instance: Guest, store: &mut Store<crate::Wasi<MyHostThing>>) -> anyhow::Result<()> {
    let thing1 = instance.thing().call_constructor(&mut *store, "Bonjour")?;
    let thing2 = instance.thing().call_constructor(&mut *store, "mon cher")?;
    let foo1 = ImportFoo { thing: thing1 };
    let foo2 = ImportFoo { thing: thing2 };

    let res = instance
        .call_test(&mut *store, &vec![foo1, foo2])?
        .into_iter()
        .map(|x| instance.thing().call_get(&mut *store, x))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        res,
        vec![
            "Bonjour Thing HostThing test HostThing HostThing.get Thing.get",
            "mon cher Thing HostThing test HostThing HostThing.get Thing.get"
        ]
    );

    Ok(())
}
