//@ args = '--disable-custom-section-link-helpers'

include!(env!("BINDINGS"));

use crate::a::x;

fn main() {
    x();
}
