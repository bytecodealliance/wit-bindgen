include!(env!("BINDINGS"));

use exports::test::resource_borrow::to_test::{Guest, GuestThing, ThingBorrow};

pub struct Test {}

export!(Test);

pub struct MyThing {
    val: u32,
}

fn get_val<'a>(v: &ThingBorrow<'a>) -> &'a u32 {
  v.get::<MyThing>().val
}

impl Guest for Test {
    type Thing = MyThing;

    fn foo(v: ThingBorrow<'_>) -> u32 {
        get_val(&v) + 2
    }
}

impl GuestThing for MyThing {
    fn new(v: u32) -> Self {
        Self { val: v + 1 }
    }
}
