//@ args = '--chainable-methods all'

// Should have no effect on exports

include!(env!("BINDINGS"));

use crate::exports::foo::bar::i::{Guest, GuestA, GuestB};
use std::cell::Cell;

struct Component;
export!(Component);
impl Guest for Component {
    type A = MyA;
    type B = MyB;
}

struct MyA {
    prop_a: Cell<u32>,
    prop_b: Cell<bool>,
}

struct MyB {
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

    fn set_a(&self, a: u32) {
        self.prop_a.set(a);
    }

    fn set_b(&self, b: bool) {
        self.prop_b.set(b);
    }

    fn do_(&self) {}
}

impl GuestB for MyB {
    fn new() -> MyB {
        MyB {
            prop_a: Cell::new(0),
            prop_b: Cell::new(false),
        }
    }

    fn set_a(&self, a: u32) {
        self.prop_a.set(a);
    }

    fn set_b(&self, b: bool) {
        self.prop_b.set(b);
    }

    fn do_(&self) {}
}
