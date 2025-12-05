include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl Guest for Component {
    fn run() {
        crate::test::resource_into_inner::to_test::test();
    }
}
