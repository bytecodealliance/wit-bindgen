//@ args = '--chainable-methods foo:bar/i#a,&foo:bar/i#b'

include!(env!("BINDINGS"));

use crate::foo::bar::i::A;
use crate::foo::bar::i::B;
use crate::foo::bar::i::C;

struct Component;
export!(Component);

impl Guest for Component {
    #[allow(unused_assignments)]
    fn run() {
        let mut my_a = A::new();
        my_a = my_a.set_a(42).set_b(true).do_();

        let my_b = B::new();
        my_b.set_a(42).set_b(true).do_();

        let my_c = C::new();
        my_c.set_a(42);
        my_c.set_b(true);
        my_c.do_();
    }
}
