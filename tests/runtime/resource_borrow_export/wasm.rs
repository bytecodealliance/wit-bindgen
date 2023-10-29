wit_bindgen::generate!({
    path: "../../tests/runtime/resource_borrow_export",
    exports: {
        world: Test,
        "test:resource-borrow-export/test": Test,
        "test:resource-borrow-export/test/thing": MyThing,
    },
});

use exports::test::resource_borrow_export::test::{Guest, GuestThing, Thing};

pub struct Test {}

pub struct MyThing {
    val: u32,
}

impl Guest for Test {
    fn foo(v: &Thing) -> u32 {
        v.val + 2
    }
}

impl GuestThing for MyThing {
    fn new(v: u32) -> Self {
        Self { val: v + 1 }
    }
}
