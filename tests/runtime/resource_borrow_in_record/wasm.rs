wit_bindgen::generate!({
    path: "../../tests/runtime/resource_borrow_in_record",
    exports: {
        world: Test,
        "test:resource-borrow-in-record/test": Test,
        "test:resource-borrow-in-record/test/thing": MyThing,
    },
});

use exports::test::resource_borrow_in_record::test::{Guest, GuestThing, OwnThing};
use test::resource_borrow_in_record::test::Foo;
use test::resource_borrow_in_record::test::Thing;

pub struct Test {}

impl Guest for Test {
    fn test(
        a: Vec<exports::test::resource_borrow_in_record::test::Foo>,
    ) -> Vec<exports::test::resource_borrow_in_record::test::OwnThing> {
        let foo = a
            .iter()
            .map(
                |a: &exports::test::resource_borrow_in_record::test::Foo| Foo {
                    thing: &a.thing.thing,
                },
            )
            .collect::<Vec<Foo>>();
        test::resource_borrow_in_record::test::test(&foo)
            .into_iter()
            .map(|a| OwnThing::new(MyThing::from_thing(a)))
            .collect()
    }
}

#[derive(Debug)]
pub struct MyThing {
    thing: Thing,
}

impl MyThing {
    pub fn from_thing(thing: Thing) -> Self {
        Self { thing }
    }
}

impl GuestThing for MyThing {
    fn new(s: String) -> Self {
        Self {
            thing: Thing::new(&format!("{} Thing", s)),
        }
    }
    fn get(&self) -> String {
        self.thing.get() + " Thing.get"
    }
}
