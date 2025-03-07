//@ args = ['--disable-run-ctors-once-workaround']

include!(env!("BINDINGS"));

fn main() {
    the::test::i::apply_the_workaround();
}
