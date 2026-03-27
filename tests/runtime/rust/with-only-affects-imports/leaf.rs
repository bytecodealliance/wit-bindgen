include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::my::inline::foo::{Guest, A};

impl Guest for Component {
    fn bar(a: A) {
        assert_eq!(a.b, 2);
    }
}
