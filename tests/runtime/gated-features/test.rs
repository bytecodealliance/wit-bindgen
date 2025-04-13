//@ args = '--features y'

include!(env!("BINDINGS"));

struct Component;

export!(Component);

use crate::exports::foo::bar::bindings::Guest;

impl Guest for Component {
    fn y() {}
    fn z() {}
}
