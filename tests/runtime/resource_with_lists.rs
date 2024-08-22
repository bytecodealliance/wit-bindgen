use std::collections::HashMap;
use wasmtime::{component::Resource, Store};

use self::test::resource_with_lists::test::{Host, HostThing, Thing};
use crate::resource_with_lists::exports::test::resource_with_lists::test::Guest;

wasmtime::component::bindgen!(in "tests/runtime/resource_with_lists");

#[derive(Default)]
pub struct MyHostThing {
    map_l: HashMap<u32, Vec<u8>>,
    next_id: u32,
}

impl Host for MyHostThing {}

impl HostThing for MyHostThing {
    fn new(&mut self, l: Vec<u8>) -> Resource<Thing> {
        let id = self.next_id;
        self.next_id += 1;
        let mut result = l.clone();
        result.extend_from_slice(" HostThing".as_bytes());
        self.map_l.insert(id, result);
        Resource::new_own(id)
    }

    fn foo(&mut self, self_: Resource<Thing>) -> Vec<u8> {
        let id = self_.rep();
        let mut list = self.map_l[&id].clone();
        list.extend_from_slice(" HostThing.foo".as_bytes());
        list
    }

    fn bar(&mut self, self_: Resource<Thing>, l: Vec<u8>) {
        let id = self_.rep();
        let mut result = l.clone();
        result.extend_from_slice(" HostThing.bar".as_bytes());
        self.map_l.insert(id, result);
    }

    fn baz(&mut self, l: Vec<u8>) -> Vec<u8> {
        let mut result = l.clone();
        result.extend_from_slice(" HostThing.baz".as_bytes());
        result
    }

    fn drop(&mut self, rep: Resource<Thing>) -> wasmtime::Result<()> {
        let id = rep.rep();
        self.map_l.remove(&id);
        Ok(())
    }
}

#[test]
fn run() -> anyhow::Result<()> {
    crate::run_test(
        "resource_with_lists",
        |linker| ResourceWithLists::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| {
            Ok(ResourceWithLists::instantiate(store, component, linker)?.interface0)
        },
        run_test,
    )
}

fn run_test(exports: Guest, store: &mut Store<crate::Wasi<MyHostThing>>) -> anyhow::Result<()> {
    let thing = exports.thing();

    let hi_encoded = "Hi".as_bytes().to_vec();
    let thing_instance = thing.call_constructor(&mut *store, &hi_encoded)?;

    let mut expected = hi_encoded.clone();
    expected.extend_from_slice(" Thing".as_bytes());
    expected.extend_from_slice(" HostThing".as_bytes());
    expected.extend_from_slice(" HostThing.foo".as_bytes());
    expected.extend_from_slice(" Thing.foo".as_bytes());
    assert_eq!(thing.call_foo(&mut *store, thing_instance)?, expected);

    let hola_encoded = "Hola".as_bytes().to_vec();
    thing.call_bar(&mut *store, thing_instance, &hola_encoded)?;

    expected = hola_encoded.clone();
    expected.extend_from_slice(" Thing.bar".as_bytes());
    expected.extend_from_slice(" HostThing.bar".as_bytes());
    expected.extend_from_slice(" HostThing.foo".as_bytes());
    expected.extend_from_slice(" Thing.foo".as_bytes());
    assert_eq!(thing.call_foo(&mut *store, thing_instance)?, expected);

    let ohayo_encoded = "Ohayo Gozaimas".as_bytes().to_vec();
    let baz_result = thing.call_baz(&mut *store, &ohayo_encoded)?;

    expected = ohayo_encoded;
    expected.extend_from_slice(" Thing.baz".as_bytes());
    expected.extend_from_slice(" HostThing.baz".as_bytes());
    expected.extend_from_slice(" Thing.baz".as_bytes());
    expected.extend_from_slice(" again".as_bytes());
    assert_eq!(baz_result, expected);

    Ok(())
}
