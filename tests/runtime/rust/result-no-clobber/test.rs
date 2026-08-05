//@ args = ['--disable-run-ctors-once-workaround']

include!(env!("BINDINGS"));

struct Test;

export!(Test);

impl exports::the::test::i::Guest for Test {
    fn no_clobber(
        _faux_result: exports::the::test::i::Result,
        _faux_option: exports::the::test::i::Option,
    ) -> Result<(), Option<bool>> {
        Err(None)
    }
}
