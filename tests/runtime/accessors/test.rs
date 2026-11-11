include!(env!("BINDINGS"));

use crate::exports::test::accessors::i::{Guest, GuestBlob};
use std::cell::{Cell, RefCell};

struct Component;

export!(Component);

static mut COUNTER: u32 = 0;
static mut BOUNDED: u32 = 0;
static mut MAX_SIZE: u64 = 1024;

impl Guest for Component {
    type Blob = MyBlob;

    fn counter() -> u32 {
        unsafe { COUNTER }
    }

    fn set_counter(value: u32) {
        unsafe { COUNTER = value }
    }

    fn read_only() -> String {
        "read only".to_string()
    }

    fn bounded() -> u32 {
        unsafe { BOUNDED }
    }

    fn set_bounded(value: u32) -> Result<(), String> {
        if value > 100 {
            return Err(format!("{value} is out of bounds"));
        }
        unsafe { BOUNDED = value }
        Ok(())
    }
}

struct MyBlob {
    contents: Vec<u8>,
    position: Cell<u64>,
    label: RefCell<String>,
}

impl GuestBlob for MyBlob {
    fn new(init: Vec<u8>) -> MyBlob {
        MyBlob {
            contents: init,
            position: Cell::new(0),
            label: RefCell::new(String::new()),
        }
    }

    fn contents(&self) -> Vec<u8> {
        self.contents.clone()
    }

    fn position(&self) -> u64 {
        self.position.get()
    }

    fn set_position(&self, value: u64) {
        self.position.set(value);
    }

    fn label(&self) -> String {
        self.label.borrow().clone()
    }

    fn set_label(&self, value: String) -> Result<(), String> {
        if value.is_empty() {
            return Err("label must not be empty".to_string());
        }
        *self.label.borrow_mut() = value;
        Ok(())
    }

    fn max_size() -> u64 {
        unsafe { MAX_SIZE }
    }

    fn set_max_size(value: u64) -> Result<(), String> {
        if value == 0 {
            return Err("max size must be nonzero".to_string());
        }
        unsafe { MAX_SIZE = value }
        Ok(())
    }
}
