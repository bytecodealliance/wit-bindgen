include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::a::Guest;

impl Guest for Component {
    fn x() {}
}
