include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::my::inline::bar::{Guest, Msg};

impl Guest for Component {
    fn bar(m: Msg) {
        assert_eq!(m.field, "hello");
    }
}
