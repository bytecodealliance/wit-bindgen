//@ args = '--features y'

include!(env!("BINDINGS"));

use crate::foo::bar::bindings::{y, z};

fn main() {
    y();
    z();
}
