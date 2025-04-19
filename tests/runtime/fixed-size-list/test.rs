include!(env!("BINDINGS"));

struct Component;

export!(Component);

impl exports::test::lists::to_test::Guest for Component {
}
