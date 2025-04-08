include!(env!("BINDINGS"));

use crate::exports::test::resource_with_lists::test::{Guest, GuestThing};
use std::cell::RefCell;

#[derive(Default)]
pub struct MyThing(RefCell<Vec<u8>>);

struct Component;

export!(Component);

impl Guest for Component {
    type Thing = MyThing;
}

impl GuestThing for MyThing {
    fn new(mut l: Vec<u8>) -> MyThing {
        l.extend_from_slice(" HostThing".as_bytes());
        MyThing(RefCell::new(l))
    }

    fn foo(&self) -> Vec<u8> {
        let mut list = self.0.borrow().clone();
        list.extend_from_slice(" HostThing.foo".as_bytes());
        list
    }

    fn bar(&self, mut l: Vec<u8>) {
        l.extend_from_slice(" HostThing.bar".as_bytes());
        *self.0.borrow_mut() = l;
    }

    fn baz(mut l: Vec<u8>) -> Vec<u8> {
        l.extend_from_slice(" HostThing.baz".as_bytes());
        l
    }
}
