include!(env!("BINDINGS"));

use crate::exports::test::resource_import_and_export::test::{Guest, GuestThing, Thing};
use std::cell::Cell;

struct Component;

export!(Component);

struct MyThing(Cell<u32>);

impl Guest for Component {
    type Thing = MyThing;
}

impl GuestThing for MyThing {
    fn new(v: u32) -> MyThing {
        MyThing(Cell::new(v + 1))
    }

    fn foo(&self) -> u32 {
        self.0.get() + 2
    }

    fn bar(&self, v: u32) {
        self.0.set(v + 3);
    }

    fn baz(a: Thing, b: Thing) -> Thing {
        let a = a.get::<MyThing>();
        let b = b.get::<MyThing>();
        Thing::new(MyThing::new(a.foo() + b.foo() + 4))
    }
}
