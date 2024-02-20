wit_bindgen::generate!({
    path: "../../tests/runtime/resource_aggregates",
    exports: {
        world: Test,
        "test:resource-aggregates/test": Test,
        "test:resource-aggregates/test/thing": MyThing,
    },
});

use test::resource_aggregates::test::{foo as import_foo, Thing};

pub struct Test {}

#[derive(Debug)]
pub struct MyThing {
    value: Option<Thing>,
}

impl exports::test::resource_aggregates::test::Guest for Test {
    fn foo(
        mut r1: exports::test::resource_aggregates::test::R1,
        r2: exports::test::resource_aggregates::test::R2,
        mut r3: exports::test::resource_aggregates::test::R3,
        mut t1: exports::test::resource_aggregates::test::T1,
        t2: exports::test::resource_aggregates::test::T2,
        v1: exports::test::resource_aggregates::test::V1,
        v2: exports::test::resource_aggregates::test::V2,
        l1: exports::test::resource_aggregates::test::L1,
        l2: exports::test::resource_aggregates::test::L2,
        o1: Option<exports::test::resource_aggregates::test::OwnThing>,
        o2: Option<&exports::test::resource_aggregates::test::Thing>,
        result1: Result<exports::test::resource_aggregates::test::OwnThing, ()>,
        result2: Result<&exports::test::resource_aggregates::test::Thing, ()>,
    ) -> u32 {
        let r1 = test::resource_aggregates::test::R1 {
            thing: Option::take(&mut r1.thing.value).unwrap(),
        };
        let r2 = test::resource_aggregates::test::R2 {
            thing: r2.thing.value.as_ref().unwrap(),
        };
        let r3 = test::resource_aggregates::test::R3 {
            thing1: r3.thing1.value.as_ref().unwrap(),
            thing2: Option::take(&mut r3.thing2.value).unwrap(),
        };
        let t1: test::resource_aggregates::test::T1 = (
            Option::take(&mut t1.0.value).unwrap(),
            test::resource_aggregates::test::R1 {
                thing: Option::take(&mut t1.1.thing.value).unwrap(),
            },
        );
        let t2: test::resource_aggregates::test::T2 = (t2.0.value.as_ref().unwrap(),);
        let v1 = test::resource_aggregates::test::V1::Thing(match v1 {
            exports::test::resource_aggregates::test::V1::Thing(mut v) => {
                Option::take(&mut v.value).unwrap()
            }
        });
        let v2 = test::resource_aggregates::test::V2::Thing(match v2 {
            exports::test::resource_aggregates::test::V2::Thing(v) => v.value.as_ref().unwrap(),
        });
        let l1 = l1
            .into_iter()
            .map(|mut v| Option::take(&mut v.value).unwrap())
            .collect::<Vec<_>>();
        let l2 = l2
            .into_iter()
            .map(|v| v.value.as_ref().unwrap())
            .collect::<Vec<_>>();
        let o1 = o1.map(|mut v| Option::take(&mut v.value).unwrap());
        let o2 = o2.map(|v| v.value.as_ref().unwrap());
        let result1 = result1.map(|mut v| Option::take(&mut v.value).unwrap());
        let result2 = result2.map(|v| v.value.as_ref().unwrap());
        import_foo(
            r1, &r2, r3, t1, &t2, v1, &v2, l1, &l2, o1, o2, result1, result2,
        ) + 4
    }
}
impl exports::test::resource_aggregates::test::GuestThing for MyThing {
    fn new(v: u32) -> Self {
        Self {
            value: Some(Thing::new(v + 1)),
        }
    }
}
