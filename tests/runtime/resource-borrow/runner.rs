include!(env!("BINDINGS"));

use crate::test::resource_borrow::to_test::{foo, Thing};

fn main() {
    assert_eq!(foo(&Thing::new(42)), 42 + 1 + 2);
}
