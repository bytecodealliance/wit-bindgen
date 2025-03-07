//@ args = ['--disable-run-ctors-once-workaround']

include!(env!("BINDINGS"));

struct Test;

export!(Test);

impl exports::the::test::i::Guest for Test {
    fn apply_the_workaround() {}
}
