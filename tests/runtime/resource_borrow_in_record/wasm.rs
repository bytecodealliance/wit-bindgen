wit_bindgen::generate!({
    path: "../../tests/runtime/resource_borrow_in_record",
    exports: {
        world: Test,
        "test:resource-borrow-in-record/test": Test,
        "test:resource-borrow-in-record/test/thing": MyThing,
    },
});

use exports::test::resource_borrow_in_record::test::{Guest, GuestThing, OwnThing};
use test::resource_borrow_in_record::test::Thing;
use test::resource_borrow_in_record::test::Foo;

pub struct Test {}

impl Guest for Test {
    fn test(
        a: wit_bindgen::rt::vec::Vec<exports::test::resource_borrow_in_record::test::Foo>,
    ) -> wit_bindgen::rt::vec::Vec<exports::test::resource_borrow_in_record::test::OwnThing> {
        let foo = a.iter()
            .map(|a: &exports::test::resource_borrow_in_record::test::Foo| {
                Foo {
                    thing: &a.thing.thing,
                }
            })
            .collect::<Vec<Foo>>();
        test::resource_borrow_in_record::test::test(&foo)
            .into_iter()
            .map(|a| {
                OwnThing::new(
                    MyThing::from_thing(a),
                ) 
            })
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
    fn new(s: wit_bindgen::rt::string::String) -> Self {
        Self {
            thing: Thing::new(&format!("{} Thing", s)),
        }
    }
    fn get(&self) -> wit_bindgen::rt::string::String {
        self.thing.get() + " Thing.get"
    }
}
