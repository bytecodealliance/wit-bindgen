include!(env!("BINDINGS"));

use crate::exports::imports::{Guest, GuestY, Y};
use std::cell::Cell;

struct Component;

export!(Component);

struct MyY(Cell<i32>);

impl Guest for Component {
    type Y = MyY;
}

impl GuestY for MyY {
    fn new(a: i32) -> MyY {
        MyY(Cell::new(a))
    }

    fn get_a(&self) -> i32 {
        self.0.get()
    }

    fn set_a(&self, a: i32) {
        self.0.set(a);
    }

    fn add(y: Y, a: i32) -> Y {
        Y::new(MyY::new(y.get::<MyY>().0.get() + a))
    }
}
