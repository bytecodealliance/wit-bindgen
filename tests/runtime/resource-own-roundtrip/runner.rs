include!(env!("BINDINGS"));

use crate::test::resource_own_roundtrip::to_test::{consume_both, num_dropped, Thing};

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        let a = Thing::new(1);
        let b = Thing::new(2);
        assert_eq!(consume_both(a, b), 3);
        assert_eq!(num_dropped(), 2);

        let a = Thing::new(3);
        let b = Thing::new(4);
        assert_eq!(consume_both(b, a), 7);
        assert_eq!(num_dropped(), 4);
    }
}
