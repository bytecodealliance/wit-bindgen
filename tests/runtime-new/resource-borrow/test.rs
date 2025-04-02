include!(env!("BINDINGS"));

use exports::test::resource_borrow::to_test::{Guest, GuestThing, ThingBorrow};

pub struct Test {}

export!(Test);

pub struct MyThing {
    val: u32,
}

impl Guest for Test {
    type Thing = MyThing;

    fn foo(v: ThingBorrow<'_>) -> u32 {
        v.get::<MyThing>().val + 2
    }
}

impl GuestThing for MyThing {
    fn new(v: u32) -> Self {
        Self { val: v + 1 }
    }
}
