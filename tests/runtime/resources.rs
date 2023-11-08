use anyhow::Result;
use std::collections::HashMap;
use wasmtime::component::ResourceAny;

wasmtime::component::bindgen!(in "tests/runtime/resources");

use imports::HostY;
use imports::Y;
use wasmtime::component::Resource;
use wasmtime::Store;

use self::exports::exports::Exports;
use self::imports::Host;

#[derive(Default)]
pub struct MyImports {
    map_a: HashMap<u32, i32>,
    next_id: u32,
}

impl HostY for MyImports {
    fn new(&mut self, a: i32) -> wasmtime::Result<wasmtime::component::Resource<Y>> {
        let id = self.next_id;
        self.next_id += 1;
        self.map_a.insert(id, a);
        Ok(Resource::new_own(id))
    }

    fn get_a(&mut self, self_: wasmtime::component::Resource<Y>) -> wasmtime::Result<i32> {
        let id = self_.rep();
        Ok(self.map_a[&id])
    }

    fn set_a(&mut self, self_: wasmtime::component::Resource<Y>, a: i32) -> wasmtime::Result<()> {
        let id = self_.rep();
        self.map_a.insert(id, a);
        Ok(())
    }

    fn add(
        &mut self,
        y: wasmtime::component::Resource<Y>,
        a: i32,
    ) -> wasmtime::Result<wasmtime::component::Resource<Y>> {
        let id = self.next_id;
        self.next_id += 1;
        let y = y.rep();
        self.map_a.insert(id, self.map_a[&y] + a);
        Ok(Resource::new_own(id))
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Y>) -> wasmtime::Result<()> {
        let id = rep.rep();
        self.map_a.remove(&id);
        Ok(())
    }
}

impl Host for MyImports {}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "resources",
        |linker| Resources::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| {
            let (u, e) = Resources::instantiate(store, component, linker)?;
            Ok((u.interface0, e))
        },
        run_test,
    )
}

fn run_test(exports: Exports, store: &mut Store<crate::Wasi<MyImports>>) -> Result<()> {
    let _ = exports.call_test_imports(&mut *store)?;

    let x = exports.x();
    let x_instance = x.call_constructor(&mut *store, 5)?;
    assert_eq!(x.call_get_a(&mut *store, x_instance)?, 5);
    x.call_set_a(&mut *store, x_instance, 10)?;
    assert_eq!(x.call_get_a(&mut *store, x_instance)?, 10);
    let z = exports.z();
    let z_instance_1 = z.call_constructor(&mut *store, 10)?;
    assert_eq!(z.call_get_a(&mut *store, z_instance_1)?, 10);

    let z_instance_2 = z.call_constructor(&mut *store, 20)?;
    assert_eq!(z.call_get_a(&mut *store, z_instance_2)?, 20);

    let x_add = x.call_add(&mut *store, x_instance, 5)?;
    assert_eq!(x.call_get_a(&mut *store, x_add)?, 15);

    let z_add = exports.call_add(&mut *store, z_instance_1, z_instance_2)?;
    assert_eq!(z.call_get_a(&mut *store, z_add)?, 30);

    ResourceAny::resource_drop(x_instance, &mut *store)?;
    ResourceAny::resource_drop(z_instance_1, &mut *store)?;
    ResourceAny::resource_drop(z_instance_2, &mut *store)?;

    Ok(())
}
