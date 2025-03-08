//@ args = '--with foo:baz/a=generate'

include!(env!("BINDINGS"));

use crate::foo::baz::a::x;

fn main() {
    x();
}
