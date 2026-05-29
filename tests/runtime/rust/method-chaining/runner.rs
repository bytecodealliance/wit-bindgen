//@ args = '--chainable-methods foo:bar/i#a'

include!(env!("BINDINGS"));

use crate::foo::bar::i::A;
use crate::foo::bar::i::B;

struct Component;
export!(Component);

impl Guest for Component {
    fn run() {
        let my_a = A::new();
        my_a.set_a(42).set_b(true).do_();

        let my_b = B::new();
        my_b.set_a(42);
        my_b.set_b(true);
        my_b.do_();
    }
}
