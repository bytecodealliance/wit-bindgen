use anyhow::Result;
use std::collections::HashMap;
use wasmtime::Store;

use test::resource_aggregates::test::{Host, HostThing, Thing, L1, L2, R1, R2, R3, T1, T2, V1, V2};

use exports::test::resource_aggregates::test::{
    R1 as TestR1, R2 as TestR2, R3 as TestR3, V1 as TestV1, V2 as TestV2,
};

wasmtime::component::bindgen!(in "tests/runtime/resource_aggregates");

#[derive(Default)]
pub struct MyHostThing {
    map_a: HashMap<u32, u32>,
    next_id: u32,
}

impl HostThing for MyHostThing {
    fn new(&mut self, v: u32) -> wasmtime::component::Resource<Thing> {
        let id = self.next_id;
        self.next_id += 1;
        self.map_a.insert(id, v + 2);
        wasmtime::component::Resource::new_own(id)
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Thing>) -> wasmtime::Result<()> {
        let id = rep.rep();
        self.map_a.remove(&id);
        Ok(())
    }
}

impl MyHostThing {
    pub fn get_value(&self, rep: wasmtime::component::Resource<Thing>) -> u32 {
        let id = rep.rep();
        self.map_a[&id]
    }
}

impl Host for MyHostThing {
    fn foo(
        &mut self,
        r1: R1,
        r2: R2,
        r3: R3,
        t1: T1,
        t2: T2,
        v1: V1,
        v2: V2,
        l1: L1,
        l2: L2,
        o1: Option<wasmtime::component::Resource<Thing>>,
        o2: Option<wasmtime::component::Resource<Thing>>,
        result1: Result<wasmtime::component::Resource<Thing>, ()>,
        result2: Result<wasmtime::component::Resource<Thing>, ()>,
    ) -> u32 {
        let res = self.get_value(r1.thing)
            + self.get_value(r2.thing)
            + self.get_value(r3.thing1)
            + self.get_value(r3.thing2)
            + self.get_value(t1.0)
            + self.get_value(t1.1.thing)
            + self.get_value(t2.0)
            + match v1 {
                V1::Thing(v) => self.get_value(v),
            }
            + match v2 {
                V2::Thing(v) => self.get_value(v),
            }
            + l1.into_iter().fold(0, |a, f| a + self.get_value(f))
            + l2.into_iter().fold(0, |a, f| a + self.get_value(f))
            + o1.map(|o| self.get_value(o)).unwrap_or_default()
            + o2.map(|o| self.get_value(o)).unwrap_or_default()
            + result1.map(|o| self.get_value(o)).unwrap_or_default()
            + result2.map(|o| self.get_value(o)).unwrap_or_default()
            + 3;
        res
    }
}

#[test]
fn run() -> Result<()> {
    crate::run_test(
        "resource_aggregates",
        |linker| ResourceAggregates::add_to_linker(linker, |x| &mut x.0),
        |store, component, linker| ResourceAggregates::instantiate(store, component, linker),
        run_test,
    )
}

fn run_test(
    instance: ResourceAggregates,
    store: &mut Store<crate::Wasi<MyHostThing>>,
) -> Result<()> {
    let mut things = vec![];
    let mut expected = 0;
    for i in 1..18 {
        let thing = instance
            .test_resource_aggregates_test()
            .thing()
            .call_constructor(&mut *store, i)?;
        things.push(thing);
        expected += i + 1 + 2;
    }

    expected += 3 + 4;

    assert_eq!(
        instance.test_resource_aggregates_test().call_foo(
            &mut *store,
            TestR1 { thing: things[0] },
            TestR2 { thing: things[1] },
            TestR3 {
                thing1: things[2],
                thing2: things[3],
            },
            (things[4], TestR1 { thing: things[5] }),
            (things[6],),
            TestV1::Thing(things[7]),
            TestV2::Thing(things[8]),
            &vec![things[9], things[10]],
            &vec![things[11], things[12]],
            Some(things[13]),
            Some(things[14]),
            Ok(things[15]),
            Ok(things[16])
        )?,
        expected
    );

    Ok(())
}
