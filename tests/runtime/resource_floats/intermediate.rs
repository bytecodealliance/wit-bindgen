include!(env!("BINDINGS"));

use exports::exports::{Float as FloatExport, GuestFloat};
use imports::Float as ImportFloat1;
use test::resource_floats::test::Float as ImportFloat2;

pub struct Test {}

export!(Test);

pub struct MyFloat {
    val: Option<ImportFloat1>,
}

impl Guest for Test {
    fn add(a: &Float, b: &Float) -> Float {
        ImportFloat2::new(a.get() + b.get() + 5.0)
    }
}

impl exports::exports::Guest for Test {
    type Float = MyFloat;
}

impl GuestFloat for MyFloat {
    fn new(v: f64) -> Self {
        Self {
            val: Some(ImportFloat1::new(v + 1.0)),
        }
    }
    fn get(&self) -> f64 {
        self.val.as_ref().unwrap().get() + 3.0
    }
    fn add(mut a: FloatExport, b: f64) -> FloatExport {
        let a = a.get_mut::<MyFloat>();
        FloatExport::new(Self::new(
            ImportFloat1::add(Option::take(&mut a.val).unwrap(), b).get() + 5.0,
        ))
    }
}
