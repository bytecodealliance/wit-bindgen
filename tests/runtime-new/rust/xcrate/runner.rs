use crate::test::xcrate::b_exports::{b, X};

include!(env!("BINDINGS"));

fn main() {
    b();
    X::new().foo();
}
