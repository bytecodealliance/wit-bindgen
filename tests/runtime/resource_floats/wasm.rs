wit_bindgen::generate!({
    path: "../../tests/runtime/resource_floats",
    exports: {
        world: Test,
        "exports/float": MyFloat,
    }
});

use exports::exports::{GuestFloat, OwnFloat};
use test::resource_floats::test::Float as ImportFloat2;
use imports::Float as ImportFloat1;

pub struct Test {}

pub struct MyFloat {
    val: Option<ImportFloat1>
}

impl Guest for Test {
    fn add(a: &Float, b: &Float) -> Float {
        ImportFloat2::new(a.get() + b.get() + 5.0)
    }
}

impl GuestFloat for MyFloat {
    fn new(v: f64) -> Self {
        Self { val: Some(ImportFloat1::new(v + 1.0)) }
    }
    fn get(&self) -> f64 {
        self.val.as_ref().unwrap().get() + 3.0
    }
    fn add(mut a: OwnFloat, b: f64) -> OwnFloat {
        OwnFloat::new(Self::new(ImportFloat1::add(Option::take(&mut a.val).unwrap(), b).get() + 5.0))
    }
}
