//@ args = ['--disable-run-ctors-once-workaround']

include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        let faux_result = false;
        let faux_option = true;
        match the::test::i::no_clobber(faux_result, faux_option) {
            Ok(_) => {}
            Err(None) => {}
            Err(Some(_)) => {}
        };
    }
}
