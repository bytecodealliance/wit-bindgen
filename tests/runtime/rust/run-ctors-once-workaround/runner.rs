//@ args = ['--disable-run-ctors-once-workaround']

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        the::test::i::apply_the_workaround();
    }
}
