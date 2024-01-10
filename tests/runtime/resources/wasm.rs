use std::cell::RefCell;

wit_bindgen::generate!({
    path: "../../tests/runtime/resources",
    exports: {
        world: Test,
        "exports": Test,
        "exports/x": ComponentX,
        "exports/z": ComponentZ,
        "exports/kebab-case": ComponentKebabCase,
    }
});

use exports::exports::OwnX;
use exports::exports::OwnKebabCase;

pub struct Test {}

pub struct ComponentX {
    val: RefCell<i32>,
}

pub struct ComponentZ {
    val: i32,
}

pub struct ComponentKebabCase {
    val: u32,
}

impl exports::exports::Guest for Test {
    fn add(a: &ComponentZ, b: &ComponentZ) -> wit_bindgen::Resource<ComponentZ> {
        wit_bindgen::Resource::new(ComponentZ { val: a.val + b.val })
    }
    fn test_imports() -> Result<(), String> {
        use imports::*;
        let y = Y::new(10);
        assert_eq!(y.get_a(), 10);
        y.set_a(20);
        assert_eq!(y.get_a(), 20);
        let y2 = Y::add(y, 20);
        assert_eq!(y2.get_a(), 40);

        // test multiple instances
        let y1 = Y::new(1);
        let y2 = Y::new(2);
        assert_eq!(y1.get_a(), 1);
        assert_eq!(y2.get_a(), 2);
        y1.set_a(10);
        y2.set_a(20);
        assert_eq!(y1.get_a(), 10);
        assert_eq!(y2.get_a(), 20);
        let y3 = Y::add(y1, 20);
        let y4 = Y::add(y2, 30);
        assert_eq!(y3.get_a(), 30);
        assert_eq!(y4.get_a(), 50);
        Ok(())
    }
}

impl exports::exports::GuestX for ComponentX {
    fn new(a: i32) -> Self {
        Self {
            val: RefCell::new(a),
        }
    }
    fn get_a(&self) -> i32 {
        *self.val.borrow()
    }
    fn set_a(&self, a: i32) {
        *self.val.borrow_mut() = a;
    }
    fn add(x: OwnX, a: i32) -> OwnX {
        x.set_a(x.get_a() + a);
        x
    }
}

impl exports::exports::GuestZ for ComponentZ {
    fn new(a: i32) -> Self {
        Self { val: a }
    }
    fn get_a(&self) -> i32 {
        self.val
    }
}

impl exports::exports::GuestKebabCase for ComponentKebabCase {
    fn new(a: u32) -> Self {
        Self { val: a }
    }
    fn get_a(&self) -> u32 {
        self.val
    }

    fn take_owned(k: OwnKebabCase) -> u32 {
        k.get_a()
    }
}
