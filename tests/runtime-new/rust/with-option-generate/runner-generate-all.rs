//@ args = '--generate-all'

include!(env!("BINDINGS"));

use crate::foo::baz::a::x;

fn main() {
    x();
}
