wit_bindgen::generate!({
    path: "../../tests/runtime/resource_borrow_in_record",
});

use exports::test::resource_borrow_in_record::test::{Guest, GuestThing, Thing as ThingExport};
use test::resource_borrow_in_record::test::Foo;
use test::resource_borrow_in_record::test::Thing;

pub struct Test {}

export!(Test);

impl Guest for Test {
    type Thing = MyThing;

    fn test(
        a: Vec<exports::test::resource_borrow_in_record::test::Foo>,
    ) -> Vec<exports::test::resource_borrow_in_record::test::Thing> {
        let foo = a
            .iter()
            .map(
                |a: &exports::test::resource_borrow_in_record::test::Foo| Foo {
                    thing: &a.thing.get::<MyThing>().thing,
                },
            )
            .collect::<Vec<Foo>>();
        test::resource_borrow_in_record::test::test(&foo)
            .into_iter()
            .map(|a| ThingExport::new(MyThing::from_thing(a)))
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
