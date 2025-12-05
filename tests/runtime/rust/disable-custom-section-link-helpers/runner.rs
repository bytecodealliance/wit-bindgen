//@ args = '--disable-custom-section-link-helpers'

include!(env!("BINDINGS"));

use crate::a::x;

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        x();
    }
}
