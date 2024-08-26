use std::collections::HashMap;
use wasmtime::{component::Resource, Store};

use self::{
    imports::{Float as ImportFloat1, Host, HostFloat},
    test::resource_floats::test::{Host as Host2, HostFloat as HostFloat2},
};

wasmtime::component::bindgen!(in "tests/runtime/resource_floats");

#[derive(Default)]
pub struct MyHostFloats {
    map_l: HashMap<u32, f64>,
    next_id: u32,
}

impl Host for MyHostFloats {}
impl HostFloat for MyHostFloats {
    fn new(&mut self, v: f64) -> wasmtime::component::Resource<ImportFloat1> {
        let id = self.next_id;
        self.next_id += 1;
        self.map_l.insert(id, v + 2.0);
        Resource::new_own(id)
    }

    fn get(&mut self, self_: wasmtime::component::Resource<ImportFloat1>) -> f64 {
        let id = self_.rep();
        self.map_l[&id] + 4.0
    }

    fn add(
        &mut self,
        a: wasmtime::component::Resource<ImportFloat1>,
        b: f64,
    ) -> wasmtime::component::Resource<ImportFloat1> {
        let id = a.rep();
        let a_value = self.map_l[&id];
        (self as &mut dyn HostFloat).new(a_value + b + 6.0)
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<ImportFloat1>) -> wasmtime::Result<()> {
        let id = rep.rep();
        self.map_l.remove(&id);
        Ok(())
    }
}
impl Host2 for MyHostFloats {}

impl HostFloat2 for MyHostFloats {
    fn new(&mut self, v: f64) -> wasmtime::component::Resource<Float> {
        let id = self.next_id;
        self.next_id += 1;
        self.map_l.insert(id, v + 1.0);
        Resource::new_own(id)
    }

    fn get(&mut self, self_: wasmtime::component::Resource<Float>) -> f64 {
        let id = self_.rep();
        self.map_l[&id] + 3.0
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Float>) -> wasmtime::Result<()> {
        let id = rep.rep();
        self.map_l.remove(&id);
        Ok(())
    }
}

#[test]
fn run() -> anyhow::Result<()> {
    crate::run_test(
        "resource_floats",
        |linker| ResourceFloats::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| ResourceFloats::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(
    instance: ResourceFloats,
    store: &mut Store<crate::Wasi<MyHostFloats>>,
) -> anyhow::Result<()> {
    // let mut float1 = MyHostFloats::default();
    // let mut float2 = MyHostFloats::default();
    // let float1 = (&mut float1 as &mut dyn HostFloat2).new(42.0)?;
    // let float2 = (&mut float2 as &mut dyn HostFloat2).new(55.0)?;
    // let float3: Resource<test::resource_floats::test::Float> = instance.call_add(&mut *store, float1, float2)?;
    // assert_eq!(0., 42. + 1. + 3. + 55. + 1. + 3. + 5. + 1.);

    let float3 = instance
        .exports()
        .float()
        .call_constructor(&mut *store, 22.0)?;
    assert_eq!(
        instance.exports().float().call_get(&mut *store, float3)?,
        22. + 1. + 2. + 4. + 3.
    );

    let res = instance
        .exports()
        .float()
        .call_add(&mut *store, float3, 7.0)?;
    assert_eq!(instance.exports().float().call_get(&mut *store, res)?, 59.0);
    Ok(())
}
