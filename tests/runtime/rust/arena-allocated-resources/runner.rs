include!(env!("BINDINGS"));

use crate::test::arena_allocated_resources::to_test::Thing;

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        let thing1 = Thing::new(3);
        let thing2 = Thing::new(5);
        assert_eq!(3, thing1.get());
        assert_eq!(5, thing2.get());
    }
}
