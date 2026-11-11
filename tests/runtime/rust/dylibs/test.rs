//@ [lang]
//@ link_shared = true

include!(env!("BINDINGS"));

use crate::exports::my::inline::{a, b};

struct Component;
export!(Component);

impl a::Guest for Component {
    fn a() {}
}

impl b::Guest for Component {
    fn b() {}
}
