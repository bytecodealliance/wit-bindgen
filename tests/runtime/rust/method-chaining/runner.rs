//@ args = '--enable-method-chaining'

include!(env!("BINDINGS"));

use crate::foo::bar::i::A;

struct Component;
export!(Component);

impl Guest for Component {
    fn run() {
        let my_a = A::new();
        my_a.set_a(42).set_b(true).do_();
    }
}
