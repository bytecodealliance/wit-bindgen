//@ args = '--with foo:baz/a=generate'

include!(env!("BINDINGS"));

use crate::foo::baz::a::x;

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        x();
    }
}
