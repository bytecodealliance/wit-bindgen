include!(env!("BINDINGS"));

use exports::test::resource_import_and_export::test::{GuestThing, Thing as ExportThing};
use std::cell::RefCell;

pub struct Test {}

export!(Test);

pub struct MyThing {
    thing: RefCell<Option<Thing>>,
}

impl Guest for Test {
    fn toplevel_export(input: Thing) -> Thing {
        toplevel_import(input)
    }
}

impl exports::test::resource_import_and_export::test::Guest for Test {
    type Thing = MyThing;
}

impl GuestThing for MyThing {
    fn new(v: u32) -> Self {
        Self {
            thing: RefCell::new(Some(Thing::new(v + 1))),
        }
    }

    fn foo(&self) -> u32 {
        let thing = self.thing.borrow();
        let thing = thing.as_ref().unwrap();
        thing.foo() + 2
    }

    fn bar(&self, v: u32) {
        let mut thing = self.thing.borrow_mut();
        let thing = thing.as_mut().unwrap();
        thing.bar(v + 3);
    }

    fn baz(a: ExportThing, b: ExportThing) -> ExportThing {
        let mut a = a.get::<MyThing>().thing.borrow_mut();
        let mut b = b.get::<MyThing>().thing.borrow_mut();
        let result =
            Thing::baz(Option::take(&mut a).unwrap(), Option::take(&mut b).unwrap()).foo() + 4;
        ExportThing::new(MyThing::new(result))
    }
}
