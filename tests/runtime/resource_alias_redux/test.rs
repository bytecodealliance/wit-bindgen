include!(env!("BINDINGS"));

use crate::exports::test::resource_alias_redux::resource_alias1 as a1;
use crate::exports::test::resource_alias_redux::resource_alias2 as a2;
use crate::exports::the_test::{Guest, Thing};

struct Component;

export!(Component);

struct MyThing(String);

impl Guest for Component {
    fn test(things: Vec<Thing>) -> Vec<Thing> {
        things
    }
}

impl a1::Guest for Component {
    type Thing = MyThing;

    fn a(f: a1::Foo) -> Vec<Thing> {
        vec![f.thing]
    }
}

impl a2::Guest for Component {
    fn b(f: a2::Foo, g: a2::Bar) -> Vec<Thing> {
        vec![f.thing, g.thing]
    }
}

impl a1::GuestThing for MyThing {
    fn new(mut msg: String) -> MyThing {
        msg.push_str(" GuestThing");
        MyThing(msg)
    }

    fn get(&self) -> String {
        let mut ret = self.0.clone();
        ret.push_str(" GuestThing.get");
        ret
    }
}
