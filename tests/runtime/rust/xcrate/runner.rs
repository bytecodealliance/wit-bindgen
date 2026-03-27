use crate::test::xcrate::b_exports::{b, X};

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        b();
        X::new().foo();
    }
}
