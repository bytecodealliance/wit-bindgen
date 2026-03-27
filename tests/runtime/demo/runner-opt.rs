//@ [lang]
//@ rustflags = '-O'

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        a::b::the_test::x();
    }
}
