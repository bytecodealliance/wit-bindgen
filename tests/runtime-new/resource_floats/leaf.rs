include!(env!("BINDINGS"));

use exports::imports::{Float as ImportFloat1, Guest, GuestFloat};
use exports::test::resource_floats::test::{Guest as Guest2, GuestFloat as GuestFloat2};

struct Component;

export!(Component);

#[derive(Default)]
pub struct MyFloat(f64);

impl Guest for Component {
    type Float = MyFloat;
}

impl GuestFloat for MyFloat {
    fn new(v: f64) -> MyFloat {
        MyFloat(v + 2.0)
    }

    fn get(&self) -> f64 {
        self.0 + 4.0
    }

    fn add(a: ImportFloat1, b: f64) -> ImportFloat1 {
        ImportFloat1::new(<MyFloat as GuestFloat>::new(a.get::<MyFloat>().0 + b + 6.0))
    }
}

impl Guest2 for Component {
    type Float = MyFloat;
}

impl GuestFloat2 for MyFloat {
    fn new(v: f64) -> MyFloat {
        MyFloat(v + 1.0)
    }

    fn get(&self) -> f64 {
        self.0 + 3.0
    }
}
