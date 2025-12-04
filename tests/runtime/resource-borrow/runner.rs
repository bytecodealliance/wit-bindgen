include!(env!("BINDINGS"));

use crate::test::resource_borrow::to_test::{foo, Thing};

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        assert_eq!(foo(&Thing::new(42)), 42 + 1 + 2);
    }
}
