include!(env!("BINDINGS"));

use exports::exports::{KebabCase, ZBorrow, X, Z};
use std::cell::RefCell;

pub struct Test {}

export!(Test);

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
    type X = ComponentX;
    type Z = ComponentZ;
    type KebabCase = ComponentKebabCase;

    fn add(a: ZBorrow<'_>, b: ZBorrow<'_>) -> Z {
        let a = a.get::<ComponentZ>();
        let b = b.get::<ComponentZ>();
        Z::new(ComponentZ { val: a.val + b.val })
    }

    fn consume(x: exports::exports::X) {
        drop(x);
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
    fn add(x: X, a: i32) -> X {
        {
            let x = x.get::<ComponentX>();
            x.set_a(x.get_a() + a);
        }
        x
    }
}

static mut NUM_DROPPED_ZS: u32 = 0;

impl exports::exports::GuestZ for ComponentZ {
    fn new(a: i32) -> Self {
        Self { val: a }
    }
    fn get_a(&self) -> i32 {
        self.val
    }

    fn num_dropped() -> u32 {
        unsafe { NUM_DROPPED_ZS + 1 }
    }
}

impl Drop for ComponentZ {
    fn drop(&mut self) {
        unsafe {
            NUM_DROPPED_ZS += 1;
        }
    }
}

impl exports::exports::GuestKebabCase for ComponentKebabCase {
    fn new(a: u32) -> Self {
        Self { val: a }
    }
    fn get_a(&self) -> u32 {
        self.val
    }

    fn take_owned(k: KebabCase) -> u32 {
        k.get::<ComponentKebabCase>().get_a()
    }
}
