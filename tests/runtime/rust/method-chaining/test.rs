//@ args = '--enable-method-chaining'

include!(env!("BINDINGS"));

use crate::exports::foo::bar::i::{Guest, GuestA};
use std::cell::Cell;

struct Component;
export!(Component);
impl Guest for Component {
    type A = MyA;
}

struct MyA {
    prop_a: Cell<u32>,
    prop_b: Cell<bool>,
}

impl GuestA for MyA {
    fn new() -> MyA {
        MyA {
            prop_a: Cell::new(0),
            prop_b: Cell::new(false),
        }
    }

    fn set_a(&self, a: u32) -> &Self {
        self.prop_a.set(a);
        self
    }

    fn set_b(&self, b: bool) -> &Self {
        self.prop_b.set(b);
        self
    }

    fn do_(&self) -> &Self {
        self
    }
}
