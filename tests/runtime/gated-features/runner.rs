//@ args = '--features y'

include!(env!("BINDINGS"));

use crate::foo::bar::bindings::{y, z};

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        y();
        z();
    }
}
